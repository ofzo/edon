use core::panic;

use anyhow::anyhow;
use oxc_allocator::Allocator;
use oxc_ast::{ast::ImportAttribute, AstKind, Visit};
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_semantic::SemanticBuilder;
use oxc_span::{GetSpan, SourceType};
use oxc_transformer::{TransformOptions, TransformTarget, Transformer};

use crate::compile::ModuleDependency;

#[derive(Debug, Default)]
struct ImportParser {
    sync_imports: Vec<String>,
    async_imports: Vec<String>,
    specifiers: Vec<String>,
    namespaces: Vec<String>,
    default_import: bool,
}

impl<'a> oxc_ast::Visit<'a> for ImportParser {
    fn visit_import_declaration(&mut self, decl: &oxc_ast::ast::ImportDeclaration<'a>) {
        let kind = AstKind::ImportDeclaration(self.alloc(decl));
        self.enter_node(kind);
        if let Some(specifiers) = &decl.specifiers {
            for specifer in specifiers {
                self.visit_import_declaration_specifier(specifer);
            }
        }
        self.sync_imports.push(decl.source.value.to_string());
        self.leave_node(kind);
    }

    fn visit_import_declaration_specifier(
        &mut self,
        specifier: &oxc_ast::ast::ImportDeclarationSpecifier,
    ) {
        use oxc_ast::ast::ImportDeclarationSpecifier;
        match specifier {
            ImportDeclarationSpecifier::ImportSpecifier(import_specifer) => {
                // println!("ImportSpecifier: {:?}", import_specifer.imported.name());
                self.specifiers
                    .push(import_specifer.imported.name().to_string());
            }
            ImportDeclarationSpecifier::ImportDefaultSpecifier(_import_specfier) => {
                // println!("ImportDefaultSpecifier: {:?}", import_specfier);
                self.default_import = true;
            }
            ImportDeclarationSpecifier::ImportNamespaceSpecifier(import_specifer) => {
                // println!("ImportNamespaceSpecifier: {:?}", import_specifer);
                self.namespaces.push(import_specifer.local.name.to_string());
            }
        }
    }
    fn visit_import_expression(&mut self, expr: &oxc_ast::ast::ImportExpression<'a>) {
        let kind = AstKind::ImportExpression(self.alloc(expr));
        self.enter_node(kind);
        if let oxc_ast::ast::Expression::StringLiteral(v) = &expr.source {
            self.async_imports.push(v.value.to_string())
        }
        self.leave_node(kind);
    }
}

pub fn compile(file_name: &str, content: &str) -> anyhow::Result<ModuleDependency> {
    let allo = Allocator::default();
    let source_type = oxc_span::SourceType::from_path(file_name).unwrap();

    let ret = oxc_parser::Parser::new(&allo, &content, source_type).parse();

    if !ret.errors.is_empty() {
        for error in ret.errors {
            let error = error.with_source_code(content.to_string());
            println!("{error:?}");
        }
        return Err(anyhow!("compile error"));
    }

    let mut import_parser = ImportParser::default();
    import_parser.visit_program(&ret.program);

    let semantic = SemanticBuilder::new(&content, source_type)
        .with_trivias(ret.trivias)
        .build(&ret.program)
        .semantic;

    let program = allo.alloc(ret.program);
    let transform_options = TransformOptions::default();

    let transformer = Transformer::new(&allo, source_type, semantic, transform_options);

    if let Err(errors) = transformer.build(program) {
        for err in errors {
            println!("{}", err);
        }
        panic!("");
    }

    let code = Codegen::<true>::new(content.len(), CodegenOptions).build(program);

    Ok(ModuleDependency {
        deps: import_parser.sync_imports,
        async_deps: import_parser.async_imports,
        specifiers: import_parser.specifiers,
        source: code,
        map: None,
        filename: file_name.to_string(),
        is_main: false,
    })
}
