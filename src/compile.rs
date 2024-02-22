use crate::{
    compile_oxc,
    graph::resolve,
    runtime::{ModuleInstance, Runtime},
};
use anyhow::anyhow;
use std::{io::Result, path::PathBuf, sync::Arc};
use v8::Isolate;

#[derive(Debug)]
pub struct ModuleDependency {
    pub deps: Vec<String>,
    pub async_deps: Vec<String>,
    pub specifiers: Vec<String>,
    pub source: String,
    pub map: Option<String>,
    pub filename: String,
    pub is_main: bool,
}

impl ModuleDependency {
    pub fn initialize(&self, isolate: &mut Isolate) -> anyhow::Result<()> {
        let graph_rc = Runtime::graph(isolate);

        {
            let graph = graph_rc.borrow();
            let module = graph.module.clone();

            let module = module.borrow();
            if module.get(&self.filename).is_some() {
                return Ok(());
            }
        }

        // {
        let state = graph_rc.borrow();
        let graph = state.table.borrow();
        for url in self.deps.iter() {
            let url = resolve(url, &self.filename);
            let dep = graph.get(&url).unwrap();
            dep.initialize(isolate)?
        }

        // }
        self.instantiate_module(isolate)?;

        Ok(())
    }
    fn instantiate_module(&self, isolate: &mut Isolate) -> anyhow::Result<()> {
        let state_rc = Runtime::state(isolate);
        let graph_rc = Runtime::graph(isolate);

        let state = state_rc.borrow_mut();
        let scope = &mut v8::HandleScope::with_context(isolate, &state.context);

        let source = v8::String::new(scope, &self.source).unwrap();
        let name = v8::String::new(scope, &self.filename).unwrap();
        let origin = v8::ScriptOrigin::new(
            scope,
            name.into(),
            0,
            0,
            false,
            123,
            name.into(),
            false,
            false,
            true,
        );

        let source = v8::script_compiler::Source::new(source, Some(&origin));
        let module = v8::script_compiler::compile_module(scope, source).unwrap();

        let module_id = module.get_identity_hash();

        let graph = graph_rc.borrow();
        graph
            .hash
            .borrow_mut()
            .insert(module_id, self.filename.clone());

        let tc_scope = &mut v8::TryCatch::new(scope);
        let result = module.instantiate_module(tc_scope, Runtime::resolve_module_callback);
        if result.is_none() {
            let expection = tc_scope.exception().unwrap();
            let msg = expection.to_rust_string_lossy(tc_scope);
            return Err(anyhow!("{}", msg));
        }

        let expose = &module.get_module_namespace();
        let v8_module = v8::Global::new(tc_scope, module);
        let expose = v8::Global::new(tc_scope, expose);

        let mut module = graph.module.borrow_mut();
        module.insert(
            self.filename.clone(),
            ModuleInstance {
                module: v8_module,
                expose,
            },
        );
        Ok(())
    }

    pub fn evaluate(&self, isolate: &mut Isolate) -> anyhow::Result<()> {
        let state_rc = Runtime::state(isolate);
        let graph_rc = Runtime::graph(isolate);

        for url in &self.deps {
            let graph = graph_rc.borrow();
            let table = graph.table.borrow();
            let url = resolve(&url, &self.filename);
            let dep = table
                .get(&url)
                .ok_or(anyhow!("table get failure `{url}`"))?;
            dep.evaluate(isolate)?;
        }

        let context = state_rc.borrow().context.clone();
        let scope = &mut v8::HandleScope::with_context(isolate, context);
        let tc_scope = &mut v8::TryCatch::new(scope);

        let state = graph_rc.borrow();
        let module = state.module.borrow_mut();
        let info = module.get(&self.filename).unwrap();

        let module = v8::Local::new(tc_scope, &info.module);
        let result = module.evaluate(tc_scope).unwrap();

        if tc_scope.has_caught() {
            let expection = tc_scope.exception().unwrap();
            return Err(anyhow!("{}", expection.to_rust_string_lossy(tc_scope)));
        }

        if result.is_promise() {
            let promise = v8::Local::<v8::Promise>::try_from(result).unwrap();
            if let v8::PromiseState::Rejected = promise.state() {
                let result = promise.result(tc_scope);
                let stack = tc_scope.stack_trace();
                return Err(anyhow!(
                    "{}\n  at {:?}",
                    result.to_rust_string_lossy(tc_scope),
                    stack
                ));
            }
        }
        Ok(())
    }
}

pub use compile_oxc::compile;
