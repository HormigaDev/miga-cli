use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use oxc_allocator::Allocator;
use oxc_ast::ast::{Statement, StringLiteral};
use oxc_codegen::{Codegen, CodegenOptions, CommentOptions};
use oxc_minifier::{CompressOptions, Minifier, MinifierOptions};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::{Atom, SourceType};
use oxc_transformer::{TransformOptions, Transformer};

pub struct CompileOptions {
    pub minify: bool,
    pub script_root: PathBuf,
    /// Maps bare module names to resolved versions for import path rewriting.
    /// For user scripts: populated from `ProjectManifest.modules`.
    /// For module code: populated from `LockedModule.resolved_deps`.
    pub dep_versions: HashMap<String, String>,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            minify: false,
            script_root: PathBuf::from("behavior/scripts"),
            dep_versions: HashMap::new(),
        }
    }
}

/// Compiles a TypeScript file to JavaScript.
///
/// `ts_path` — source file on disk (used for parsing and reading).
/// `dest_path` — output location (used to calculate correct import prefix).
pub fn compile_file(ts_path: &Path, dest_path: &Path, opts: &CompileOptions) -> Result<String> {
    let source = std::fs::read_to_string(ts_path)
        .with_context(|| format!("Cannot read {}", ts_path.display()))?;
    compile_source(&source, ts_path, dest_path, opts)
}

pub fn compile_source(
    source: &str,
    path: &Path,
    dest_path: &Path,
    opts: &CompileOptions,
) -> Result<String> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(path).unwrap_or_else(|_| SourceType::ts());

    let parser_ret = Parser::new(&allocator, source, source_type).parse();
    if !parser_ret.errors.is_empty() {
        let messages: Vec<String> = parser_ret.errors.iter().map(|e| e.to_string()).collect();
        return Err(anyhow::anyhow!(
            "TypeScript parse error in {}:\n{}",
            path.display(),
            messages.join("\n")
        ));
    }

    let mut program = parser_ret.program;

    let scoping = SemanticBuilder::new()
        .build(&program)
        .semantic
        .into_scoping();

    let transform_options = TransformOptions {
        typescript: oxc_transformer::TypeScriptOptions::default(),
        ..TransformOptions::default()
    };

    let transform_ret = Transformer::new(&allocator, path, &transform_options)
        .build_with_scoping(scoping, &mut program);

    if !transform_ret.errors.is_empty() {
        let messages: Vec<String> = transform_ret.errors.iter().map(|e| e.to_string()).collect();
        return Err(anyhow::anyhow!(
            "Transform error in {}:\n{}",
            path.display(),
            messages.join("\n")
        ));
    }

    rewrite_imports(
        &allocator,
        &mut program,
        dest_path,
        &opts.script_root,
        &opts.dep_versions,
    );

    if opts.minify {
        let minifier_options = MinifierOptions {
            mangle: None,
            compress: Some(CompressOptions::default()),
        };
        Minifier::new(minifier_options).minify(&allocator, &mut program);
    }

    Ok(Codegen::new()
        .with_options(CodegenOptions {
            minify: true,
            comments: CommentOptions {
                annotation: false,
                jsdoc: false,
                normal: false,
                ..Default::default()
            },
            ..CodegenOptions::default()
        })
        .build(&program)
        .code)
}

fn rewrite_imports<'a>(
    allocator: &'a Allocator,
    program: &mut oxc_ast::ast::Program<'a>,
    dest_path: &Path,
    script_root: &Path,
    dep_versions: &HashMap<String, String>,
) {
    let prefix = import_prefix(dest_path, script_root);

    for stmt in &mut program.body {
        match stmt {
            Statement::ImportDeclaration(decl) => {
                rewrite_source(allocator, &mut decl.source, &prefix, dep_versions);
            }
            Statement::ExportNamedDeclaration(decl) => {
                if let Some(source) = &mut decl.source {
                    rewrite_source(allocator, source, &prefix, dep_versions);
                }
            }
            Statement::ExportAllDeclaration(decl) => {
                rewrite_source(allocator, &mut decl.source, &prefix, dep_versions);
            }
            _ => {}
        }
    }
}

fn rewrite_source<'a>(
    allocator: &'a Allocator,
    source: &mut StringLiteral<'a>,
    prefix: &str,
    dep_versions: &HashMap<String, String>,
) {
    let value = source.value.as_str();

    if value.contains("minecraft") || value.starts_with('.') || value.starts_with("@oxc-project") {
        return;
    }

    let (module_name, sub_path) = if value.contains('/') {
        let mut parts = value.splitn(2, '/');
        (parts.next().unwrap(), parts.next())
    } else {
        (value, None)
    };

    let new_path = if let Some(version) = dep_versions.get(module_name) {
        match sub_path {
            Some(sub) => format!("{}libs/{}/v{}/{}.js", prefix, module_name, version, sub),
            None => {
                format!(
                    "{}libs/{}/v{}/{}.js",
                    prefix, module_name, version, module_name
                )
            }
        }
    } else {
        // No version resolved — fall back to unversioned path.
        match sub_path {
            Some(sub) => format!("{}libs/{}/{}.js", prefix, module_name, sub),
            None => format!("{}libs/{}/{}.js", prefix, module_name, module_name),
        }
    };

    source.value = Atom::from(allocator.alloc_str(&new_path));
}

fn import_prefix(dest_path: &Path, script_root: &Path) -> String {
    let current_dir = dest_path.parent().unwrap_or_else(|| Path::new(""));

    match current_dir.strip_prefix(script_root) {
        Ok(relative) => {
            let depth = relative.components().count();
            if depth == 0 {
                "./".to_string()
            } else {
                "../".repeat(depth)
            }
        }
        Err(_) => "./".to_string(),
    }
}
