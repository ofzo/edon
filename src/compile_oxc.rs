use std::path::Path;

use anyhow::bail;
use oxc_allocator::Allocator;
use oxc_ast::{AstKind, Trivias, Visit};
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_transformer::{TransformOptions, Transformer};

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
        let mut err = vec![];
        for report in ret.errors {
            err.push(format!("{}", report.with_source_code(content.to_string())));
        }
        bail!("{}", err.join("\n"));
    }

    let mut import_parser = ImportParser::default();
    import_parser.visit_program(&ret.program);

    let program = allo.alloc(ret.program);
    let transform_options = TransformOptions::default();

    let trivias = Trivias::default();
    let transformer = Transformer::new(
        &allo,
        Path::new(file_name),
        source_type,
        content,
        &trivias,
        transform_options,
    );

    if let Err(errors) = transformer.build(program) {
        let err = errors
            .iter()
            .map(|err| format!("{}", err))
            .collect::<Vec<_>>()
            .join("\n");
        bail!("{err}");
    }

    let code = Codegen::<true>::new(
        file_name,
        content,
        CodegenOptions {
            enable_source_map: true,
            enable_typescript: false, // allow output typescript code
        },
    )
    .build(program);

    Ok(ModuleDependency {
        deps: import_parser.sync_imports,
        async_deps: import_parser.async_imports,
        specifiers: import_parser.specifiers,
        source: code.source_text,
        map: code
            .source_map
            .map(|m| m.to_json_string().unwrap_or_default()),
        filename: file_name.to_string(),
        is_main: false,
    })
}
