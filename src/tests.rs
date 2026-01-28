use super::*;
use std::collections::{HashMap, HashSet};

fn find_subcommand<'a>(cmd: &'a Command, name: &str) -> Option<&'a Command> {
    cmd.get_subcommands().find(|sub| sub.get_name() == name)
}

fn path_tokens(path: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut remaining = path;
    while let Some(start) = remaining.find('<') {
        if let Some(end) = remaining[start + 1..].find('>') {
            let token = &remaining[start + 1..start + 1 + end];
            let name = token.split(':').nth(1).unwrap_or(token);
            out.push(name.to_string());
            remaining = &remaining[start + 1 + end + 1..];
        } else {
            break;
        }
    }
    out
}

fn dummy_value_for_param(name: &str) -> String {
    let name = name.to_ascii_lowercase();
    if name.contains("slug") || name.contains("workspace") {
        return "ckrwl".to_string();
    }
    if name.contains("id") {
        return "00000000-0000-0000-0000-000000000000".to_string();
    }
    if name.contains("key") {
        return "CKR-1".to_string();
    }
    "test".to_string()
}

#[test]
fn command_tree_loads() {
    let tree = command_tree::load_command_tree();
    assert!(tree.version > 0);
    assert!(!tree.resources.is_empty());
    assert!(tree.base_path.starts_with('/'));
}

#[test]
fn command_tree_param_paths_match() {
    let tree = command_tree::load_command_tree();
    for res in &tree.resources {
        for op in &res.ops {
            let tokens = path_tokens(&op.path);
            let params: Vec<String> = op.params.iter().map(|p| p.name.clone()).collect();
            assert_eq!(tokens.len(), params.len(), "{} {}", res.name, op.name);
            for token in tokens {
                assert!(
                    params.contains(&token),
                    "missing param {} for {} {}",
                    token,
                    res.name,
                    op.name
                );
            }
        }
    }
}

#[test]
fn command_tree_has_unique_ops() {
    let tree = command_tree::load_command_tree();
    for res in &tree.resources {
        let mut seen = HashSet::new();
        for op in &res.ops {
            assert!(seen.insert(op.name.clone()), "dup op {} {}", res.name, op.name);
        }
    }
}

#[test]
fn cli_includes_all_ops() {
    let tree = command_tree::load_command_tree();
    let cli = build_cli(&tree);
    for res in &tree.resources {
        let res_cmd = find_subcommand(&cli, &res.name).expect("missing resource");
        for op in &res.ops {
            find_subcommand(res_cmd, &op.name)
                .unwrap_or_else(|| panic!("missing op {} {}", res.name, op.name));
        }
    }
}

#[test]
fn build_path_substitutes_tokens() {
    let tree = command_tree::load_command_tree();
    for res in &tree.resources {
        for op in &res.ops {
            let mut params = HashMap::new();
            for param in &op.params {
                params.insert(param.name.clone(), dummy_value_for_param(&param.name));
            }
            let path = build_path(&op.path, &params).expect("build path");
            assert!(!path.contains('<'), "unsubstituted token {}", path);
            assert!(!path.contains('>'), "unsubstituted token {}", path);
        }
    }
}

#[test]
fn join_url_preserves_trailing_slash() {
    let url = join_url("https://example.com/", "/api/v1/", "users/");
    assert_eq!(url, "https://example.com/api/v1/users/");
}

#[test]
fn split_base_url_variants() {
    let (api, path) = split_base_url("https://example.com", "/api/v1").expect("split");
    assert_eq!(api, "https://example.com");
    assert_eq!(path, "/api/v1");

    let (api, path) = split_base_url("https://example.com/ckrwl", "/api/v1").expect("split");
    assert_eq!(api, "https://example.com");
    assert_eq!(path, "/ckrwl");
}

#[test]
fn parse_query_pair_validation() {
    let (k, v) = parse_query_pair("a=b").expect("parse");
    assert_eq!(k, "a");
    assert_eq!(v, "b");
    assert!(parse_query_pair("a=").is_err());
    assert!(parse_query_pair("=b").is_err());
    assert!(parse_query_pair("ab").is_err());
}

#[test]
fn workspace_param_detection() {
    assert!(is_workspace_param("slug"));
    assert!(is_workspace_param("workspace"));
    assert!(is_workspace_param("workspace_slug"));
    assert!(is_workspace_param("workspaceSlug"));
    assert!(!is_workspace_param("project_id"));
}
