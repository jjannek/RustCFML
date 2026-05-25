//! Cloudflare Workers fetch handler.

#![cfg(target_arch = "wasm32")]

use crate::embedded_vfs::EmbeddedVfs;
use crate::scopes;
use crate::WorkerConfig;
use cfml_codegen::compiler::CfmlCompiler;
use cfml_common::dynamic::CfmlValue;
use cfml_common::vfs::Vfs;
use cfml_compiler::{parser::Parser, tag_parser};
use cfml_stdlib::builtins::{get_builtin_functions, get_builtins};
use cfml_vm::web::resolve_file;
use cfml_vm::{CfmlVirtualMachine, ServerState};
use indexmap::IndexMap;
use std::path::PathBuf;
use std::sync::Arc;
use worker::*;

/// Cloudflare Workers fetch handler entry point.
///
/// Expected to be invoked from the host project's `#[event(fetch)]` function:
///
/// ```ignore
/// #[event(fetch)]
/// pub async fn main(req: Request, env: Env, ctx: Context) -> Result<Response> {
///     let config = cfml_worker::WorkerConfig::new(CFML_FILES, "/app");
///     cfml_worker::handle_fetch(req, env, ctx, &config).await
/// }
/// ```
pub async fn handle_fetch(
    mut req: Request,
    _env: Env,
    _ctx: Context,
    config: &WorkerConfig,
) -> Result<Response> {
    let vfs: Arc<dyn Vfs> = Arc::new(EmbeddedVfs::new(
        config.embedded_files,
        config.virtual_root.to_string(),
    ));

    let path = req.path();
    let doc_root = PathBuf::from(config.virtual_root);
    let welcome = if config.welcome_files.is_empty() {
        vec!["index.cfm".to_string()]
    } else {
        config.welcome_files.clone()
    };
    let cfml_exts = if config.cfml_extensions.is_empty() {
        vec!["cfm".to_string(), "cfc".to_string()]
    } else {
        config.cfml_extensions.clone()
    };

    let resolved = match resolve_file(&doc_root, &path, vfs.as_ref(), &welcome, &cfml_exts) {
        Some(r) => r,
        None => return Response::error("Not Found", 404),
    };

    let script_name = resolved.script_name.clone();
    let path_info = resolved.path_info.clone();
    let file_path = resolved.file_path.to_string_lossy().to_string();

    // Build CGI/URL/Form/Cookie scopes
    let (globals, http_request_data) =
        match scopes::build_from_request(&mut req, &script_name, &path_info).await {
            Ok(p) => p,
            Err(e) => return Response::error(format!("scope build error: {e:?}"), 500),
        };

    // Register D1 datasources for this request. The D1Driver impl lives in
    // crate::d1_driver and is wired up in step 8.
    if !config.d1_datasources.is_empty() {
        // Placeholder: D1 driver implementation lands in step 8.
    }

    // ServerState lives per-isolate so application/session/bytecode caches
    // survive within a single isolate. We don't currently reuse the same
    // ServerState across `handle_fetch` calls in this isolate — see the
    // module-level TODO; that's wired in step 7.
    let server_state = ServerState::with_production(config.production_mode);

    // Execute
    let response_data = match run_cfml(&file_path, vfs, globals, http_request_data, &server_state) {
        Ok(r) => r,
        Err(msg) => {
            let mut headers = Headers::new();
            let _ = headers.set("Content-Type", "text/plain; charset=utf-8");
            return Ok(Response::ok(format!("CFML error:\n\n{msg}"))?
                .with_status(500)
                .with_headers(headers));
        }
    };

    // Map response
    if let Some(redirect) = response_data.redirect_url {
        return Response::redirect(redirect.parse().map_err(|_| Error::BadEncoding)?);
    }

    let status = response_data.status.unwrap_or(200);
    let content_type = response_data
        .content_type
        .unwrap_or_else(|| "text/html; charset=utf-8".into());

    let headers = Headers::new();
    headers.set("Content-Type", &content_type)?;
    for (k, v) in response_data.headers {
        headers.append(&k, &v)?;
    }

    Ok(Response::ok(response_data.output)?
        .with_status(status)
        .with_headers(headers))
}

struct ResponseData {
    output: String,
    status: Option<u16>,
    content_type: Option<String>,
    headers: Vec<(String, String)>,
    redirect_url: Option<String>,
}

fn run_cfml(
    file_path: &str,
    vfs: Arc<dyn Vfs>,
    extra_globals: IndexMap<String, CfmlValue>,
    http_request_data: CfmlValue,
    server_state: &ServerState,
) -> std::result::Result<ResponseData, String> {
    // Read source (compile path uses VFS via compile_file_cached, but we
    // also want the source for the first-compile path when the file isn't
    // yet cached).
    let source = vfs
        .read_to_string(file_path)
        .map_err(|e| format!("read {file_path}: {e}"))?;

    let processed = if tag_parser::has_cfml_tags(&source) {
        tag_parser::tags_to_script(&source)
    } else {
        source
    };

    let ast = Parser::new(processed)
        .parse()
        .map_err(|e| format!("parse error [{}:{}] {}", e.line, e.column, e.message))?;

    let program = CfmlCompiler::new().compile(ast);

    let mut vm = CfmlVirtualMachine::new(program);
    vm.vfs = vfs;
    vm.base_template_path = Some(file_path.to_string());
    vm.source_file = Some(file_path.to_string());

    for (name, value) in get_builtins() {
        vm.globals.insert(name, value);
    }
    for (name, func) in get_builtin_functions() {
        vm.builtins.insert(name, func);
    }

    vm.globals
        .entry("url".to_string())
        .or_insert_with(|| CfmlValue::strukt(IndexMap::new()));
    vm.globals
        .entry("cgi".to_string())
        .or_insert_with(|| CfmlValue::strukt(IndexMap::new()));
    vm.globals
        .entry("form".to_string())
        .or_insert_with(|| CfmlValue::strukt(IndexMap::new()));

    for (name, value) in extra_globals {
        vm.globals.insert(name, value);
    }

    vm.apply_cfconfig(&server_state.cfconfig);
    vm.server_state = Some(server_state.clone());
    vm.http_request_data = Some(http_request_data);

    // Cookie-based session id (read from globals["cookie"]["cfid"]) — wired
    // in step 7 alongside KvSessionStore.

    let result = vm.execute_with_lifecycle();
    let result = match result {
        Err(e) if e.message == "__cflocation_redirect" || e.message == "__cfabort" => {
            Ok(CfmlValue::Null)
        }
        other => other,
    };

    let output = vm.output_buffer.clone();
    let status = vm.response_status.map(|(c, _)| c);
    let content_type = vm.response_content_type.clone();
    let headers = vm.response_headers.clone();
    let redirect_url = vm.redirect_url.clone();

    match result {
        Ok(_) => Ok(ResponseData {
            output,
            status,
            content_type,
            headers,
            redirect_url,
        }),
        Err(e) => Err(format!("{}\n\nOutput so far:\n{}", e, output)),
    }
}
