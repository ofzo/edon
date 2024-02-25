use std::{
    cell::RefCell, collections::HashMap, ffi::c_void, future::poll_fn, num::NonZeroI32,
    path::PathBuf, pin::Pin, rc::Rc, task::Poll,
};

use crate::{compile, graph::DependencyGraph};
use futures::{stream::FuturesUnordered, Future, StreamExt};
use tokio::sync::mpsc::{self, Receiver, Sender};
use v8::{Isolate, OwnedIsolate};

mod asynchronous;
mod constants;
mod init;
mod static_fn;

pub use asynchronous::AsynchronousKind;
type Async = Pin<Box<dyn Future<Output = Poll<AsynchronousKind>>>>;

#[derive(Debug)]
pub struct ModuleInstance {
    pub module: v8::Global<v8::Module>,
    pub expose: v8::Global<v8::Value>,
}

#[derive(Debug)]
pub struct RuntimeGraph {
    pub table: Rc<RefCell<DependencyGraph>>,
    pub module: Rc<RefCell<HashMap<String, ModuleInstance>>>,
    pub hash: Rc<RefCell<HashMap<NonZeroI32, String>>>,
}

#[derive(Debug)]
pub struct RuntimeState {
    pub context: v8::Global<v8::Context>,
    pub pending_ops: FuturesUnordered<Async>,
    pub sender: Sender<usize>,
    pub receiver: Receiver<usize>,
}
/**
# Ts Runtime
*/
pub struct Runtime {
    pub isolate: v8::OwnedIsolate,
    pub sender: Sender<usize>,
}

impl Runtime {
    fn isolate() -> (OwnedIsolate, v8::Global<v8::Context>) {
        let platform = v8::new_default_platform(2, true).make_shared();
        v8::V8::initialize_platform(platform);
        v8::V8::initialize();

        let params = v8::CreateParams::default();
        let mut isolate = v8::Isolate::new(params);

        isolate.set_host_import_module_dynamically_callback(Self::dynamically_import);

        let global_context = {
            let scope = &mut v8::HandleScope::new(isolate.as_mut());
            let context = Runtime::init_global(scope);
            v8::Global::new(scope, context)
        };

        (isolate, global_context)
    }
    pub fn from(graph: DependencyGraph) -> Self {
        let (mut isolate, global_context) = Self::isolate();

        let (sender, receiver) = mpsc::channel::<usize>(1024);

        isolate.set_data(
            constants::ASYNC_STATE_SLOT,
            Rc::into_raw(Rc::new(RefCell::new(RuntimeState {
                context: global_context,
                sender: sender.clone(),
                receiver,
                pending_ops: FuturesUnordered::new(),
            }))) as *mut c_void,
        );

        isolate.set_data(
            constants::ASYNC_GRAPH_SLOT,
            Rc::into_raw(Rc::new(RefCell::new(RuntimeGraph {
                table: Rc::new(RefCell::new(graph)),
                module: Default::default(),
                hash: Default::default(),
            }))) as *mut c_void,
        );

        Self { isolate, sender }
    }

    pub fn state(isolate: &Isolate) -> Rc<RefCell<RuntimeState>> {
        let state_ptr =
            isolate.get_data(constants::ASYNC_STATE_SLOT) as *const RefCell<RuntimeState>;

        let state_rc = unsafe { Rc::from_raw(state_ptr) };
        let state = state_rc.clone();
        Rc::into_raw(state_rc);
        state
    }

    pub fn graph(isolate: &Isolate) -> Rc<RefCell<RuntimeGraph>> {
        let state_ptr =
            isolate.get_data(constants::ASYNC_GRAPH_SLOT) as *const RefCell<RuntimeGraph>;

        let state_rc = unsafe { Rc::from_raw(state_ptr) };
        let state = state_rc.clone();
        Rc::into_raw(state_rc);
        state
    }

    fn bootstrap(&mut self, entry: &String) -> anyhow::Result<()> {
        let isolate = self.isolate.as_mut();
        let state_rc = Self::state(isolate);
        let graph_rc = Self::graph(isolate);

        // bootstrap.js
        let mut bootstrap_js = PathBuf::new();
        bootstrap_js.push("bootstrap.ts");
        let dependency = compile::compile(
            &bootstrap_js.to_string_lossy().to_string(),
            &include_str!("../../bootstrap/main.ts").to_string(),
        )?;

        dependency.initialize(isolate)?;
        dependency.evaluate(isolate)?;

        //
        let graph = graph_rc.borrow();
        let module = graph.module.borrow();
        let info = module
            .get(&bootstrap_js.to_string_lossy().to_string())
            .unwrap();

        let state = state_rc.borrow();
        let scope = &mut v8::HandleScope::with_context(isolate, &state.context);

        let expose = v8::Local::new(scope, &info.expose);
        let obj = expose.to_object(scope).unwrap();

        let default = v8::String::new(scope, "default").unwrap();
        let default = obj.get(scope, default.into()).unwrap();

        let fun = v8::Local::<v8::Function>::try_from(default).unwrap();
        let tc_scope = &mut v8::TryCatch::new(scope);

        let this = v8::Object::new(tc_scope);
        let timer = v8::Object::new(tc_scope);

        Self::set_func(tc_scope, timer, "send", Self::timer_send);
        Self::set_obj(tc_scope, this, "timer", timer);

        let entry = v8::String::new(tc_scope, &entry).unwrap();
        fun.call(tc_scope, this.into(), &[entry.into()]);
        Ok(())
    }

    pub async fn run(&mut self, entry: &String) -> anyhow::Result<()> {
        self.bootstrap(entry)?;

        let isolate = self.isolate.as_mut();
        let state_rc = Self::state(isolate);

        loop {
            if {
                let state = state_rc.borrow();
                state.pending_ops.is_empty()
            } {
                break Ok(());
            }
            poll_fn(|cx| loop {
                let result = {
                    let mut state = state_rc.borrow_mut();
                    let result = state.pending_ops.poll_next_unpin(cx);
                    if Poll::Pending == result {
                        continue;
                    }
                    result
                };
                if let Poll::Ready(Some(Poll::Ready(op))) = result {
                    match op.exec(isolate) {
                        Ok(v) => break v,
                        Err(err) => eprintln!("{err:?}"),
                    }
                    break Poll::Ready(());
                }
                break Poll::Pending;
            })
            .await;
        }
    }

    async fn import(isolate: &mut Isolate, source: &String, base: &String) -> anyhow::Result<()> {
        let graph_rc = Self::graph(isolate);
        let graph = graph_rc.borrow();
        let mut table = graph.table.borrow_mut();

        table.append(source, base).await
    }
}
