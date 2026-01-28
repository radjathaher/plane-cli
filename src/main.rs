mod command_tree;
mod http;
#[cfg(test)]
mod tests;

use anyhow::{Context, Result, anyhow};
use clap::{Arg, ArgAction, Command};
use command_tree::{CommandTree, Operation, Param};
use http::{HttpClient, ensure_success};
use serde_json::{Value, json};
use std::{collections::HashMap, env, fs, io::Write};

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let tree = command_tree::load_command_tree();
    let cli = build_cli(&tree);
    let matches = cli.get_matches();

    if let Some(matches) = matches.subcommand_matches("list") {
        return handle_list(&tree, matches);
    }
    if let Some(matches) = matches.subcommand_matches("describe") {
        return handle_describe(&tree, matches);
    }
    if let Some(matches) = matches.subcommand_matches("tree") {
        return handle_tree(&tree, matches);
    }
    if let Some(matches) = matches.subcommand_matches("request") {
        return handle_request(&tree, &matches);
    }

    let include_deprecated = matches.get_flag("include-deprecated");

    let api_key = env::var("PLANE_API_KEY").context("PLANE_API_KEY missing")?;
    let (api_url, base_path) = resolve_api_base(&tree)?;

    let pretty = matches.get_flag("pretty");
    let raw = matches.get_flag("raw");

    let (res_name, res_matches) = matches
        .subcommand()
        .ok_or_else(|| anyhow!("resource required"))?;
    let (op_name, op_matches) = res_matches
        .subcommand()
        .ok_or_else(|| anyhow!("operation required"))?;

    let op = find_op(&tree, res_name, op_name)
        .ok_or_else(|| anyhow!("unknown command {res_name} {op_name}"))?;

    if op.deprecated && !include_deprecated {
        return Err(anyhow!("deprecated endpoint; re-run with --include-deprecated"));
    }

    let params = collect_path_params(op, op_matches)?;
    let path = build_path(&op.path, &params)?;
    let url = join_url(&api_url, &base_path, &path);

    let query = build_query_params(op_matches)?;
    let body = read_body(op_matches)?;

    let client = HttpClient::new(api_key)?;
    let response = client.execute(&op.method, &url, &query, body)?;

    let output = if raw {
        json!({
            "status": response.status,
            "headers": response.headers,
            "body": response.body,
        })
    } else {
        response.body
    };

    write_output(&output, pretty)?;
    ensure_success(response.status, &output)?;
    Ok(())
}

fn build_cli(tree: &CommandTree) -> Command {
    let mut cmd = Command::new("plane")
        .about("Plane CLI (auto-generated)")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg(
            Arg::new("pretty")
                .long("pretty")
                .global(true)
                .action(ArgAction::SetTrue)
                .help("Pretty-print JSON output"),
        )
        .arg(
            Arg::new("raw")
                .long("raw")
                .global(true)
                .action(ArgAction::SetTrue)
                .help("Return full response with status + headers"),
        )
        .arg(
            Arg::new("include-deprecated")
                .long("include-deprecated")
                .global(true)
                .action(ArgAction::SetTrue)
                .help("Enable deprecated endpoints"),
        )
        .arg(
            Arg::new("query")
                .long("query")
                .global(true)
                .action(ArgAction::Append)
                .value_name("KEY=VALUE")
                .help("Append query parameter (repeatable)"),
        )
        .arg(
            Arg::new("fields")
                .long("fields")
                .global(true)
                .value_name("FIELDS")
                .help("Comma-separated response fields"),
        )
        .arg(
            Arg::new("expand")
                .long("expand")
                .global(true)
                .action(ArgAction::Append)
                .value_name("EXPAND")
                .help("Expand related fields (repeatable)"),
        )
        .arg(
            Arg::new("per-page")
                .long("per-page")
                .global(true)
                .value_name("N")
                .help("Pagination: per_page"),
        )
        .arg(
            Arg::new("cursor")
                .long("cursor")
                .global(true)
                .value_name("CURSOR")
                .help("Pagination: cursor"),
        )
        .arg(
            Arg::new("body-json")
                .long("body-json")
                .global(true)
                .value_name("JSON")
                .help("JSON body payload"),
        )
        .arg(
            Arg::new("body-file")
                .long("body-file")
                .global(true)
                .value_name("PATH")
                .help("JSON body payload from file"),
        );

    cmd = cmd.subcommand(
        Command::new("list")
            .about("List resources and operations")
            .arg(
                Arg::new("json")
                    .long("json")
                    .action(ArgAction::SetTrue)
                    .help("Emit machine-readable JSON"),
            ),
    );

    cmd = cmd.subcommand(
        Command::new("describe")
            .about("Describe a specific operation")
            .arg(Arg::new("resource").required(true))
            .arg(Arg::new("op").required(true))
            .arg(
                Arg::new("json")
                    .long("json")
                    .action(ArgAction::SetTrue)
                    .help("Emit machine-readable JSON"),
            ),
    );

    cmd = cmd.subcommand(
        Command::new("tree").about("Show full command tree").arg(
            Arg::new("json")
                .long("json")
                .action(ArgAction::SetTrue)
                .help("Emit machine-readable JSON"),
        ),
    );

    cmd = cmd.subcommand(
        Command::new("request")
            .about("Raw request escape hatch")
            .arg(Arg::new("method").required(true))
            .arg(Arg::new("path").required(true)),
    );

    for resource in &tree.resources {
        let mut res_cmd = Command::new(resource.name.clone())
            .about(resource.name.clone())
            .subcommand_required(true)
            .arg_required_else_help(true);
        for op in &resource.ops {
            let mut op_cmd = Command::new(op.name.clone())
                .about(format!("{} {}", op.method, op.path));
            if op.deprecated {
                op_cmd = op_cmd.hide(true);
            }
            for param in &op.params {
                op_cmd = op_cmd.arg(build_param_arg(param));
            }
            res_cmd = res_cmd.subcommand(op_cmd);
        }
        cmd = cmd.subcommand(res_cmd);
    }

    cmd
}

fn build_param_arg(param: &Param) -> Arg {
    let mut arg = Arg::new(param.name.clone())
        .long(param.flag.clone())
        .value_name(param.name.clone());
    if !is_workspace_param(&param.name) {
        arg = arg.required(true);
    }
    arg
}

fn handle_list(tree: &CommandTree, matches: &clap::ArgMatches) -> Result<()> {
    let include_deprecated = matches.get_flag("include-deprecated");
    if matches.get_flag("json") {
        let mut out = Vec::new();
        for res in &tree.resources {
            let ops: Vec<Value> = res
                .ops
                .iter()
                .filter(|op| include_deprecated || !op.deprecated)
                .map(|op| {
                    json!({
                        "name": op.name,
                        "method": op.method,
                        "path": op.path,
                        "deprecated": op.deprecated,
                    })
                })
                .collect();
            out.push(json!({"resource": res.name, "ops": ops}));
        }
        write_output(&Value::Array(out), true)?;
        return Ok(());
    }

    for res in &tree.resources {
        write_stdout_line(&res.name)?;
        for op in &res.ops {
            if op.deprecated && !include_deprecated {
                continue;
            }
            let suffix = if op.deprecated { " (deprecated)" } else { "" };
            write_stdout_line(&format!("  {} {}{}", op.name, op.method, suffix))?;
        }
    }
    Ok(())
}

fn handle_describe(tree: &CommandTree, matches: &clap::ArgMatches) -> Result<()> {
    let resource = matches
        .get_one::<String>("resource")
        .ok_or_else(|| anyhow!("resource required"))?;
    let op_name = matches
        .get_one::<String>("op")
        .ok_or_else(|| anyhow!("operation required"))?;

    let op = find_op(tree, resource, op_name)
        .ok_or_else(|| anyhow!("unknown command {resource} {op_name}"))?;

    if matches.get_flag("json") {
        write_output(&serde_json::to_value(op)?, true)?;
        return Ok(());
    }

    write_stdout_line(&format!("{} {}", resource, op.name))?;
    write_stdout_line(&format!("  method: {}", op.method))?;
    write_stdout_line(&format!("  path: {}", op.path))?;
    write_stdout_line(&format!("  deprecated: {}", op.deprecated))?;
    if !op.params.is_empty() {
        write_stdout_line("  params:")?;
        for param in &op.params {
            write_stdout_line(&format!("    --{}", param.flag))?;
        }
    }
    Ok(())
}

fn handle_tree(tree: &CommandTree, matches: &clap::ArgMatches) -> Result<()> {
    if matches.get_flag("json") {
        write_output(&serde_json::to_value(tree)?, true)?;
        return Ok(());
    }
    write_stdout_line("Run with --json for machine-readable output.")?;
    Ok(())
}

fn handle_request(tree: &CommandTree, matches: &clap::ArgMatches) -> Result<()> {
    let api_key = env::var("PLANE_API_KEY").context("PLANE_API_KEY missing")?;
    let (api_url, base_path) = resolve_api_base(&tree)?;

    let method = matches
        .get_one::<String>("method")
        .ok_or_else(|| anyhow!("method required"))?;
    let path = matches
        .get_one::<String>("path")
        .ok_or_else(|| anyhow!("path required"))?;

    let url = if path.starts_with("http://") || path.starts_with("https://") {
        path.to_string()
    } else if path.starts_with('/') {
        format!("{}{}", api_url.trim_end_matches('/'), path)
    } else {
        join_url(&api_url, &base_path, path)
    };

    let query = build_query_params(matches)?;
    let body = read_body(matches)?;

    let client = HttpClient::new(api_key)?;
    let response = client.execute(method, &url, &query, body)?;

    let output = if matches.get_flag("raw") {
        json!({
            "status": response.status,
            "headers": response.headers,
            "body": response.body,
        })
    } else {
        response.body
    };

    write_output(&output, matches.get_flag("pretty"))?;
    ensure_success(response.status, &output)?;
    Ok(())
}

fn find_op<'a>(tree: &'a CommandTree, res: &str, op: &str) -> Option<&'a Operation> {
    tree.resources
        .iter()
        .find(|r| r.name == res)
        .and_then(|r| r.ops.iter().find(|o| o.name == op))
}

fn collect_path_params(op: &Operation, matches: &clap::ArgMatches) -> Result<HashMap<String, String>> {
    let mut params = HashMap::new();
    for param in &op.params {
        let mut value = matches.get_one::<String>(&param.name).cloned();
        if value.is_none() && is_workspace_param(&param.name) {
            value = env::var("PLANE_WORKSPACE").ok();
        }
        let value = value.ok_or_else(|| anyhow!("missing required argument --{}", param.flag))?;
        params.insert(param.name.clone(), value);
    }
    Ok(params)
}

fn is_workspace_param(name: &str) -> bool {
    matches!(name, "slug" | "workspace" | "workspace_slug" | "workspaceSlug")
}

fn build_path(template: &str, params: &HashMap<String, String>) -> Result<String> {
    let mut out = String::new();
    let mut cursor = 0;
    let bytes = template.as_bytes();

    while let Some(start) = find_byte(bytes, b'<', cursor) {
        let end = find_byte(bytes, b'>', start + 1).ok_or_else(|| anyhow!("invalid path template"))?;
        out.push_str(&template[cursor..start]);
        let token = &template[start + 1..end];
        let name = token.split(':').nth(1).unwrap_or(token);
        let value = params
            .get(name)
            .ok_or_else(|| anyhow!("missing value for {name}"))?;
        out.push_str(value);
        cursor = end + 1;
    }

    out.push_str(&template[cursor..]);
    Ok(out)
}

fn find_byte(haystack: &[u8], needle: u8, start: usize) -> Option<usize> {
    haystack
        .iter()
        .enumerate()
        .skip(start)
        .find_map(|(idx, byte)| if *byte == needle { Some(idx) } else { None })
}

fn join_url(base: &str, base_path: &str, path: &str) -> String {
    let base = base.trim_end_matches('/');
    let base_path = base_path.trim_matches('/');
    let path = path.trim_start_matches('/');

    if base_path.is_empty() {
        format!("{base}/{path}")
    } else if path.is_empty() {
        format!("{base}/{base_path}")
    } else {
        format!("{base}/{base_path}/{path}")
    }
}

fn build_query_params(matches: &clap::ArgMatches) -> Result<Vec<(String, String)>> {
    let mut params = Vec::new();

    if let Some(values) = matches.get_many::<String>("query") {
        for value in values {
            let (key, val) = parse_query_pair(value)?;
            params.push((key, val));
        }
    }

    if let Some(fields) = matches.get_one::<String>("fields") {
        params.push(("fields".to_string(), fields.clone()));
    }

    if let Some(expands) = matches.get_many::<String>("expand") {
        for expand in expands {
            params.push(("expand".to_string(), expand.clone()));
        }
    }

    if let Some(per_page) = matches.get_one::<String>("per-page") {
        params.push(("per_page".to_string(), per_page.clone()));
    }

    if let Some(cursor) = matches.get_one::<String>("cursor") {
        params.push(("cursor".to_string(), cursor.clone()));
    }

    Ok(params)
}

fn parse_query_pair(input: &str) -> Result<(String, String)> {
    let mut parts = input.splitn(2, '=');
    let key = parts.next().unwrap_or_default().trim();
    let value = parts.next().unwrap_or_default().trim();
    if key.is_empty() || value.is_empty() {
        return Err(anyhow!("invalid query param: {input}"));
    }
    Ok((key.to_string(), value.to_string()))
}

fn read_body(matches: &clap::ArgMatches) -> Result<Option<Value>> {
    let body_json = matches.get_one::<String>("body-json");
    let body_file = matches.get_one::<String>("body-file");

    if body_json.is_some() && body_file.is_some() {
        return Err(anyhow!("use only one of --body-json or --body-file"));
    }

    if let Some(raw) = body_json {
        let value: Value = serde_json::from_str(raw).context("invalid JSON body")?;
        return Ok(Some(value));
    }

    if let Some(path) = body_file {
        let raw = fs::read_to_string(path).context("read body file")?;
        let value: Value = serde_json::from_str(&raw).context("invalid JSON body file")?;
        return Ok(Some(value));
    }

    Ok(None)
}

fn write_output(value: &Value, pretty: bool) -> Result<()> {
    if pretty {
        write_stdout_line(&serde_json::to_string_pretty(value)?)?;
    } else {
        write_stdout_line(&serde_json::to_string(value)?)?;
    }
    Ok(())
}

fn write_stdout_line(value: &str) -> Result<()> {
    let mut out = std::io::stdout().lock();
    if let Err(err) = out.write_all(value.as_bytes()) {
        if err.kind() == std::io::ErrorKind::BrokenPipe {
            std::process::exit(0);
        }
        return Err(err.into());
    }
    if let Err(err) = out.write_all(b"\n") {
        if err.kind() == std::io::ErrorKind::BrokenPipe {
            std::process::exit(0);
        }
        return Err(err.into());
    }
    Ok(())
}

fn resolve_api_base(tree: &CommandTree) -> Result<(String, String)> {
    if let Ok(base_url) = env::var("PLANE_BASE_URL") {
        return split_base_url(&base_url, &tree.base_path);
    }

    let api_url = env::var("PLANE_API_URL").unwrap_or_else(|_| "https://api.plane.so".to_string());
    let base_path = env::var("PLANE_API_BASE_PATH").unwrap_or_else(|_| tree.base_path.clone());
    Ok((api_url, base_path))
}

fn split_base_url(base_url: &str, default_path: &str) -> Result<(String, String)> {
    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(anyhow!("PLANE_BASE_URL empty"));
    }

    let scheme_idx = trimmed
        .find("://")
        .ok_or_else(|| anyhow!("PLANE_BASE_URL must include scheme"))?;
    let path_idx = trimmed[scheme_idx + 3..]
        .find('/')
        .map(|idx| idx + scheme_idx + 3);

    let api_url = match path_idx {
        Some(idx) => trimmed[..idx].to_string(),
        None => trimmed.to_string(),
    };

    let base_path = match path_idx {
        Some(idx) => {
            let mut path = trimmed[idx..].to_string();
            if path.is_empty() {
                path = default_path.to_string();
            }
            if !path.starts_with('/') {
                path.insert(0, '/');
            }
            path
        }
        None => default_path.to_string(),
    };

    Ok((api_url, base_path))
}
