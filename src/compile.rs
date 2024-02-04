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
    pub specifier: Vec<String>,
    pub source: String,
    pub map: Option<String>,
    pub filename: String,
    pub is_main: bool,
}

impl ModuleDependency {
    pub fn initialize(&self, isolate: &mut Isolate) -> Option<()> {
        let graph_rc = Runtime::graph(isolate);

        {
            let graph = graph_rc.borrow();
            let module = graph.module.clone();

            let module = module.borrow();
            if module.get(&self.filename).is_some() {
                return Some(());
            }
        }

        // {
        self.deps.iter().for_each(|url| {
            let state = graph_rc.borrow();
            let graph = state.table.borrow();
            let url = resolve(url, &self.filename);
            let dep = graph.get(&url).unwrap();
            dep.initialize(isolate);
        });

        // }
        self.instantiate_module(isolate);

        return Some(());
    }
    fn instantiate_module(&self, isolate: &mut Isolate) {
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
        module
            .instantiate_module(tc_scope, Runtime::resolve_module_callback)
            .unwrap();
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
    }

    pub fn evaluate(&self, isolate: &mut Isolate) {
        let state_rc = Runtime::state(isolate);
        let graph_rc = Runtime::graph(isolate);

        self.deps.iter().for_each(|url| {
            let graph = graph_rc.borrow();
            let table = graph.table.borrow();
            let url = resolve(url, &self.filename);
            let dep = table
                .get(&url)
                .expect(&format!("table get failure `{url}`"));
            dep.evaluate(isolate);
        });

        let context = state_rc.borrow().context.clone();
        let scope = &mut v8::HandleScope::with_context(isolate, context);
        let tc_scope = &mut v8::TryCatch::new(scope);

        let state = graph_rc.borrow();
        let module = state.module.borrow_mut();
        let info = module.get(&self.filename).unwrap();

        let module = v8::Local::new(tc_scope, &info.module);
        let result = module.evaluate(tc_scope).unwrap();

        if result.is_promise() {
            let promise = v8::Local::<v8::Promise>::try_from(result).unwrap();
            match promise.state() {
                v8::PromiseState::Rejected => {
                    println!("evaluate fail: {}", self.filename);
                }
                _ => {}
            }
        }
    }
}

pub use compile_oxc::compile;
// pub fn compile(file_name: &str, source_text: &str) -> anyhow::Result<ModuleDependency> {
//     return compile_oxc::compile(file_name, source_text);
//     // return compile_swc::compile(file_name, source_text);
// }
