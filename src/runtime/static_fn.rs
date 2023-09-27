use std::{task::Poll, time::Duration};

use super::{asynchronous::AsynchronousKind, Runtime};
use crate::{graph::resolve, runner::console::console_format};
use url::Url;

impl Runtime {
    pub fn resolve<'s>(
        context: v8::Local<'s, v8::Context>,
        specifier: v8::Local<'s, v8::String>,
        _import_assertions: v8::Local<'s, v8::FixedArray>,
        referrer: v8::Local<'s, v8::Module>,
    ) -> Option<v8::Local<'s, v8::Module>> {
        let scope = &mut unsafe { v8::CallbackScope::new(context) };
        // let state_rc = Self::state(scope);
        let graph_rc = Self::graph(scope);

        let state = graph_rc.borrow();
        let module_id = referrer.get_identity_hash();

        let hash = state.hash.borrow();
        let url = hash.get(&module_id).unwrap();

        let specifier = specifier.to_rust_string_lossy(scope);
        let url = Url::parse(url).unwrap();
        let url = url.join(&specifier).unwrap();
        let url = url.as_str().to_string();

        let module = state.module.borrow();
        let info = module.get(&url).unwrap();
        let module = v8::Local::new(scope, &info.module);

        return Some(module);
    }

    pub fn dynamically_import<'a>(
        scope: &mut v8::HandleScope<'a>,
        _host_defined_options: v8::Local<'a, v8::Data>,
        resource: v8::Local<'a, v8::Value>,
        specifier: v8::Local<'a, v8::String>,
        _import_assertions: v8::Local<'a, v8::FixedArray>,
    ) -> Option<v8::Local<'a, v8::Promise>> {
        let state_rc = Self::state(scope);
        let resource = resource.to_rust_string_lossy(scope).to_string();
        let specifier = specifier.to_rust_string_lossy(scope).to_string();

        let resolver = v8::PromiseResolver::new(scope).unwrap();
        let promise = resolver.get_promise(scope);

        let resolver = v8::Global::new(scope, resolver);
        let state = state_rc.borrow();

        state.pending_ops.push(Box::pin(async move {
            Poll::Ready(AsynchronousKind::Import((
                resolve(&specifier, &resource),
                resolver,
            )))
        }));

        let builder = v8::FunctionBuilder::new(Runtime::promise_error);
        let err_handler = v8::FunctionBuilder::<v8::Function>::build(builder, scope).unwrap();

        promise.catch(scope, err_handler)
    }

    pub fn promise_error(
        scope: &mut v8::HandleScope,
        info: v8::FunctionCallbackArguments,
        mut rv: v8::ReturnValue,
    ) {
        let error = console_format(scope, &info.get(0), 0);
        println!("Uncaught {error}");
        rv.set(v8::Number::new(scope, 0f64).into())
    }
    pub fn timer_send(
        scope: &mut v8::HandleScope,
        info: v8::FunctionCallbackArguments,
        mut rv: v8::ReturnValue,
    ) {
        let state_rc = Runtime::state(scope);
        let id = info
            .get(0)
            .to_number(scope)
            .unwrap()
            .number_value(scope)
            .unwrap();
        let delay = info
            .get(1)
            .to_number(scope)
            .unwrap()
            .number_value(scope)
            .unwrap();

        let state = state_rc.borrow();
        state.pending_ops.push(Box::pin(async move {
            let delay = Duration::from_millis(delay as u64);
            tokio::time::sleep(delay).await;

            Poll::Ready(AsynchronousKind::Operation(id as u32))
        }));
    }
}