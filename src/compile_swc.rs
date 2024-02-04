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

#[derive(Default, Debug)]
struct ImportParser {
    sync_imports: Vec<String>,
    async_imports: Vec<String>,
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
pub fn compile(file_name: &str, source_text: &str) -> anyhow::Result<ModuleDependency> {
    return compile_oxc::compile(file_name, source_text);

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
        source_text.to_string(),
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
        .map_err(|e| anyhow!("parse {file_name} fail"))?;
    // .expect(&format!("parse {file_name} fail"));

    let mut import_parser = ImportParser::default();
    parsed.visit_with(&mut import_parser);

    let config = Config {
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
    };
    let result = GLOBALS.set(&Default::default(), || {
        compiler.run(|| {
            compiler
                .process_js_file(
                    fm,
                    &handler,
                    &Options {
                        config,
                        ..Default::default()
                    },
                )
                // .map_err(|e| anyhow!("compiler error"))
                .expect("compiler error")
        })
    });

    Ok(ModuleDependency {
        deps: import_parser.sync_imports,
        async_deps: import_parser.async_imports,
        specifier: vec![],
        source: result.code,
        map: result.map,
        filename: file_name.to_string(),
        is_main: false,
    })
}
