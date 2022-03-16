use parser::{lexer::Lexer, StringInput, Syntax};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
};
use swc::{
    config::{Config, JscConfig, ModuleConfig, Options, SourceMapsConfig},
    ecmascript::ast::{Decl, EsVersion, ModuleItem, Pat, Stmt},
};
use swc_common::{
    errors::{ColorConfig, Handler},
    FileName, SourceMap,
};
use swc_ecma_parser as parser;

pub fn swc_main(code: &str) -> String {
    let cm = Arc::<SourceMap>::default();
    let handler = Arc::new(Handler::with_tty_emitter(
        ColorConfig::Auto,
        true,
        false,
        Some(cm.clone()),
    ));
    let c = swc::Compiler::new(cm.clone());
    let fm = cm.new_source_file(
        FileName::Real(Path::new("script.js").into()),
        code.to_string(),
    );
    // let fm = cm.load_file(Path::new(filename)).unwrap();

    let lexer = Lexer::new(
        Syntax::Es(Default::default()),
        Default::default(),
        StringInput::from(&*fm),
        None,
    );
    let mut parser = parser::Parser::new_from(lexer);

    for err in parser.take_errors() {
        err.into_diagnostic(&handler).emit();
    }
    let module = parser
        .parse_module()
        .map_err(|err| {
            err.into_diagnostic(&handler).emit();
        })
        .unwrap();
    for module_item in module.body {
        if let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) = module_item {
            for decl in &var.decls {
                if let Pat::Ident(ident) = &decl.name {
                    println!("ident name: {}", ident.id.sym);
                }
            }
        }
    }
    let result = c
        .process_js_file(
            fm,
            &handler,
            &Options {
                config: Config {
                    jsc: JscConfig {
                        target: Some(EsVersion::Es2016),
                        ..Default::default()
                    },
                    module: Some(ModuleConfig::CommonJs(Default::default())),
                    source_maps: Some(SourceMapsConfig::Bool(true)),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();
    // println!("{}", result.code);

    let mut file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(PathBuf::from("output/main.js"))
        .unwrap();

    write!(file, "{}", result.code).unwrap();
    write!(file, "{}", "\n//# sourceMappingURL=main.js.map").unwrap();
    if let Some(map) = result.map {
        fs::write(PathBuf::from("output/main.js.map"), map.as_bytes()).unwrap();
    }
    println!("swc end");
    return result.code;
}
