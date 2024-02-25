use anyhow::anyhow;
use futures::Future;
use std::task::Poll;
use v8::Isolate;

use super::Runtime;

#[derive(Debug, PartialEq)]
pub enum AsynchronousKind {
    Import((String, v8::Global<v8::PromiseResolver>)),
    Operation(u32),
    // Callback(impl Future<Output = anyhow::Result<()>>),
}

impl AsynchronousKind {
    pub fn exec(&self, isolate: &mut Isolate) -> anyhow::Result<Poll<()>> {
        match self {
            AsynchronousKind::Operation(id) => Self::operation(isolate, id.clone()),
            AsynchronousKind::Import((source, resolver)) => Self::import(isolate, source, resolver),
            // AsynchronousKind::Callback(f) => f.await,
        }
    }

    fn operation(isolate: &mut Isolate, id: u32) -> anyhow::Result<Poll<()>> {
        let state_rc = Runtime::state(isolate);

        let context = {
            let state = state_rc.borrow();
            state.context.clone()
        };
        let scope = &mut v8::HandleScope::with_context(isolate, context);
        let name = v8::String::new(scope, &format!("{id}.js")).unwrap();
        let origin = v8::ScriptOrigin::new(
            scope,
            name.into(),
            0,
            0,
            false,
            id as i32,
            name.into(),
            false,
            false,
            false,
        );
        let source = v8::String::new(scope, &format!("globalThis.exec({id});")).unwrap();
        let tc_scope = &mut v8::TryCatch::new(scope);
        let script = v8::Script::compile(tc_scope, source, Some(&origin))
            .ok_or(anyhow!("compile script failure"))?;
        script
            .run(tc_scope)
            .ok_or(anyhow!("run script failure {}", id))?;

        if tc_scope.has_caught() {
            panic!("exec error");
        }
        Ok(Poll::Ready(()))
    }
    fn import(
        isolate: &mut Isolate,
        source: &String,
        resolver: &v8::Global<v8::PromiseResolver>,
    ) -> anyhow::Result<Poll<()>> {
        let state_rc = Runtime::state(isolate);
        let graph_rc = Runtime::graph(isolate);

        let context = {
            let state = state_rc.borrow();
            state.context.clone()
        };

        if {
            let graph = graph_rc.borrow();
            let table = graph.table.borrow();
            table.get(source).is_none()
        } {
            let base = format!("");
            let _ = futures::executor::block_on(Runtime::import(isolate, source, &base));
        };

        let graph = graph_rc.borrow();
        let table = graph.table.borrow();
        let dep = table
            .get(source)
            .ok_or(anyhow!("source `{}` not found", source))?;

        if dep.initialize(isolate).is_ok() {
            dep.evaluate(isolate)?;

            let scope = &mut v8::HandleScope::with_context(isolate, context);
            let tc_scope = &mut v8::TryCatch::new(scope);

            let resolver = resolver.open(tc_scope);

            let graph = graph_rc.borrow();
            let module = graph.module.borrow();
            if let Some(instance) = module.get(source) {
                let expose = v8::Local::new(tc_scope, &instance.expose);
                let obj = expose.to_object(tc_scope).unwrap();
                resolver.resolve(tc_scope, obj.into());
            };
        };

        let module = graph.module.borrow();
        if module.get(source).is_none() {
            let t = table.get(source).unwrap();
            t.initialize(isolate)?;
        }

        Ok(Poll::Ready(()))
    }
}
