use std::{path::PathBuf, sync::Arc};
use swc::{
    config::{Config, JscConfig, ModuleConfig, Options, SourceMapsConfig},
    Compiler,
};
use swc_common::{
    errors::{ColorConfig, Handler},
    FileName, SourceMap, GLOBALS,
};
use swc_ecma_ast::{CallExpr, Callee, EsVersion, Expr, Lit, ModuleDecl};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax};
use swc_ecma_visit::{Visit, VisitWith};
use v8::Isolate;

use crate::{
    graph::resolve,
    runtime::{ModuleInstance, Runtime},
};

struct ImportParser {
    sync_imports: Vec<String>,
    async_imports: Vec<String>,
}

impl ImportParser {
    pub fn new() -> Self {
        Self {
            sync_imports: vec![],
            async_imports: vec![],
        }
    }
}

impl Visit for ImportParser {
    fn visit_call_expr(&mut self, call_expr: &CallExpr) {
        if !matches!(call_expr.callee, Callee::Import(_)) {
            return;
        }
        if !call_expr.args.is_empty() {
            let arg = &call_expr.args[0];
            if let Expr::Lit(Lit::Str(v)) = &*arg.expr {
                self.async_imports.push(v.value.to_string());
            }
        }
    }
    fn visit_module_decl(&mut self, module_decl: &ModuleDecl) {
        match module_decl {
            ModuleDecl::Import(import) => {
                if import.type_only {
                    return;
                }
                self.sync_imports.push(import.src.value.to_string());
            }
            ModuleDecl::ExportNamed(export) => {
                if let Some(src) = &export.src {
                    self.sync_imports.push(src.value.to_string());
                }
            }
            ModuleDecl::ExportAll(export) => {
                self.sync_imports.push(export.src.value.to_string());
            }
            _ => {}
        }
    }
}

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
            .instantiate_module(tc_scope, Runtime::resolve)
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
            let dep = table.get(url).unwrap();
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

pub fn compile(file_name: &str, source: &str) -> ModuleDependency {
    let cm = Arc::<SourceMap>::default();
    let handler = Arc::new(Handler::with_tty_emitter(
        ColorConfig::Auto,
        true,
        false,
        Some(cm.clone()),
    ));
    let compiler = Compiler::new(cm.clone());

    let fm = cm.new_source_file(
        match url::Url::parse(file_name) {
            Ok(url) => FileName::Url(url),
            Err(_) => FileName::Real(PathBuf::from(file_name)),
        },
        source.to_string(),
    );

    let lexer = Lexer::new(
        Syntax::Typescript(Default::default()),
        EsVersion::latest(),
        StringInput::from(&*fm),
        None,
    );
    let mut parser = Parser::new_from(lexer);

    for err in parser.take_errors() {
        err.into_diagnostic(&handler).emit();
    }

    let parsed = parser
        .parse_module()
        .map_err(|e| e.into_diagnostic(&handler).emit())
        .expect(&format!("parse {file_name} fail"));

    let mut import_parser = ImportParser::new();
    parsed.visit_with(&mut import_parser);

    let result = GLOBALS.set(&Default::default(), || {
        compiler.run(|| {
            compiler
                .process_js_file(
                    fm,
                    &handler,
                    &Options {
                        config: Config {
                            jsc: JscConfig {
                                target: Some(EsVersion::Es2022),
                                syntax: Some(Syntax::Typescript(Default::default())),
                                ..Default::default()
                            },
                            module: Some(ModuleConfig::Es6(
                                swc_ecma_transforms_module::EsModuleConfig {
                                    resolve_fully: true,
                                },
                            )),
                            source_maps: Some(SourceMapsConfig::Bool(false)),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                )
                .expect("compiler error")
        })
    });

    // let filename = path.file_name().unwrap().to_str().unwrap();
    // let mut file = fs::OpenOptions::new()
    //     .write(true)
    //     .truncate(true)
    //     .create(true)
    //     .open(PathBuf::from(format!("output/{filename}.js")))
    //     .unwrap();

    // if let Some(map) = &result.map {
    //     fs::write(
    //         PathBuf::from(format!("output/{filename}.js.map")),
    //         map.as_bytes(),
    //     )
    //     .unwrap();
    // }
    ModuleDependency {
        deps: import_parser.sync_imports,
        async_deps: import_parser.async_imports,
        specifier: vec![],
        source: result.code,
        map: result.map,
        filename: file_name.to_string(),
        is_main: false,
    }
}
