#![doc = "AST-based Rust use statement scanner for Kiva architecture guards."]

use std::collections::BTreeSet;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use quote::ToTokens;
use syn::visit::Visit;
use syn::{ItemUse, UseTree, Visibility};

#[derive(Default)]
struct UseCollector {
    statements: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for UseCollector {
    fn visit_item_use(&mut self, node: &'ast ItemUse) {
        let prefix = visibility_prefix(&node.vis);
        for path in use_tree_paths(&node.tree, "") {
            self.statements.insert(format!("{prefix} {path};"));
        }
    }
}

fn visibility_prefix(visibility: &Visibility) -> String {
    match visibility {
        Visibility::Inherited => "use".to_owned(),
        Visibility::Public(_) => "pub use".to_owned(),
        Visibility::Restricted(restricted) => {
            let path = restricted
                .path
                .to_token_stream()
                .to_string()
                .replace(' ', "");
            if restricted.in_token.is_some() {
                format!("pub(in {path}) use")
            } else {
                format!("pub({path}) use")
            }
        }
    }
}

fn use_tree_paths(tree: &UseTree, prefix: &str) -> Vec<String> {
    match tree {
        UseTree::Path(path) => {
            let next_prefix = join_path(prefix, &path.ident.to_string());
            use_tree_paths(&path.tree, &next_prefix)
        }
        UseTree::Name(name) => vec![join_path(prefix, &name.ident.to_string())],
        UseTree::Rename(rename) => {
            vec![format!(
                "{} as {}",
                join_path(prefix, &rename.ident.to_string()),
                rename.rename
            )]
        }
        UseTree::Glob(_) => vec![join_path(prefix, "*")],
        UseTree::Group(group) => group
            .items
            .iter()
            .flat_map(|item| use_tree_paths(item, prefix))
            .collect(),
    }
}

fn join_path(prefix: &str, segment: &str) -> String {
    if prefix.is_empty() {
        segment.to_owned()
    } else {
        format!("{prefix}::{segment}")
    }
}

fn collect_rust_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    if path.is_file() {
        if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path.to_path_buf());
        }
        return Ok(());
    }
    if !path.is_dir() {
        return Err(format!(
            "scan path is not a file or directory: {}",
            path.display()
        ));
    }
    for entry_result in fs::read_dir(path)
        .map_err(|error| format!("failed to read directory {}: {error}", path.display()))?
    {
        let entry = entry_result.map_err(|error| {
            format!(
                "failed to read directory entry in {}: {error}",
                path.display()
            )
        })?;
        collect_rust_files(&entry.path(), files)?;
    }
    Ok(())
}

fn parse_file(path: &Path) -> Result<BTreeSet<String>, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let syntax = syn::parse_file(&text)
        .map_err(|error| format!("failed to parse {} as Rust: {error}", path.display()))?;
    let mut collector = UseCollector::default();
    collector.visit_file(&syntax);
    Ok(collector.statements)
}

fn execute(args: impl IntoIterator<Item = OsString>) -> Result<(), String> {
    let mut roots: Vec<PathBuf> = args.into_iter().map(PathBuf::from).collect();
    if roots.is_empty() {
        return Err("usage: kiva-rust-use-scan <rust-file-or-directory> ...".to_owned());
    }
    roots.sort();
    let mut files = Vec::new();
    for root in &roots {
        collect_rust_files(root, &mut files)?;
    }
    files.sort();
    files.dedup();
    let mut statements = BTreeSet::new();
    for file in files {
        statements.extend(parse_file(&file)?);
    }
    for statement in statements {
        println!("{statement}");
    }
    Ok(())
}

fn main() -> ExitCode {
    match execute(env::args_os().skip(1)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
