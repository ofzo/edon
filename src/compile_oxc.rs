use anyhow::anyhow;
use oxc_allocator::Allocator;
use oxc_ast::{AstKind, Visit};
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_semantic::SemanticBuilder;
use oxc_span::{GetSpan, SourceType};
use oxc_transformer::{TransformOptions, TransformTarget, Transformer};
use tracing::Instrument;

use crate::compile::ModuleDependency;

#[derive(Debug, Default)]
struct ImportParser {
    sync_imports: Vec<String>,
    async_imports: Vec<String>,
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
                println!("ImportDeclarationSpecifier: {:?}", import_specifer.span);
            }
            ImportDeclarationSpecifier::ImportDefaultSpecifier(import_specfier) => {
                println!("ImportDeclarationSpecifier: {:?}", import_specfier.span);
            }
            ImportDeclarationSpecifier::ImportNamespaceSpecifier(import_specifer) => {
                println!("ImportDeclarationSpecifier: {:?}", import_specifer.span);
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

pub fn compile(file_name: &str, source_text: &str) -> anyhow::Result<ModuleDependency> {
    let allo = Allocator::default();
    let source_type = oxc_span::SourceType::from_path(file_name).unwrap();

    let ret = oxc_parser::Parser::new(&allo, &source_text, source_type).parse();

    if !ret.errors.is_empty() {
        for error in ret.errors {
            let error = error.with_source_code(source_text.to_string());
            println!("{error:?}");
        }
        return Err(anyhow!("compile error"));
    }

    let mut pass = ImportParser::default();
    pass.visit_program(&ret.program);

    let semantic = SemanticBuilder::new(&source_text, source_type)
        .with_trivias(ret.trivias)
        .build(&ret.program)
        .semantic;

    let program = allo.alloc(ret.program);
    let transform_options = TransformOptions::default();

    Transformer::new(&allo, source_type, semantic, transform_options)
        .build(program)
        .unwrap();

    let code = Codegen::<false>::new(source_text.len(), CodegenOptions).build(program);

    Ok(ModuleDependency {
        deps: pass.sync_imports,
        async_deps: pass.async_imports,
        specifier: vec![],
        source: code,
        map: None,
        filename: file_name.to_string(),
        is_main: false,
    })
}
