use std::task::Poll;

use v8::Isolate;

use super::Runtime;

#[derive(Debug, PartialEq)]
pub enum AsynchronousKind {
    Operation(u32),
    Import((String, v8::Global<v8::PromiseResolver>)),
}

impl AsynchronousKind {
    pub fn exec(&self, isolate: &mut Isolate) -> Poll<()> {
        match self {
            AsynchronousKind::Operation(id) => Self::operation(isolate, id.clone()),
            AsynchronousKind::Import((specifier, resolver)) => {
                Self::import(isolate, specifier, resolver)
            }
        }
    }
    fn operation(isolate: &mut Isolate, id: u32) -> Poll<()> {
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
        let script = v8::Script::compile(scope, source, Some(&origin)).unwrap();
        script.run(scope).unwrap();
        return Poll::Ready(());
    }
    fn import(
        isolate: &mut Isolate,
        specifier: &String,
        resolver: &v8::Global<v8::PromiseResolver>,
    ) -> Poll<()> {
        let state_rc = Runtime::state(isolate);
        let graph_rc = Runtime::graph(isolate);

        let context = {
            let state = state_rc.borrow();
            state.context.clone()
        };

        if {
            let graph = graph_rc.borrow();
            let table = graph.table.borrow();
            table.get(specifier).is_none()
        } {
            let base = format!("");
            let _ = futures::executor::block_on(Runtime::import(isolate, specifier, &base));
        };

        let graph = graph_rc.borrow();
        let table = graph.table.borrow();
        let dep = table.get(specifier).unwrap();

        if dep.initialize(isolate).is_some() {
            dep.evaluate(isolate);

            let scope = &mut v8::HandleScope::with_context(isolate, context);
            let tc_scope = &mut v8::TryCatch::new(scope);

            let resolver = resolver.open(tc_scope);

            let graph = graph_rc.borrow();
            let module = graph.module.borrow();
            if let Some(instance) = module.get(specifier) {
                let expose = v8::Local::new(tc_scope, &instance.expose);
                let obj = expose.to_object(tc_scope).unwrap();
                resolver.resolve(tc_scope, obj.into());
            };
        };

        return Poll::Ready(());
    }
}
