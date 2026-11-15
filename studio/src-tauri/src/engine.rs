// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};

pub struct EngineState {
    pub running: Arc<AtomicBool>,
    pub child: Mutex<Option<Child>>,
}

fn engine_binary_names() -> &'static [&'static str] {
    #[cfg(target_os = "windows")]
    {
        &["open-ontologies-x86_64-pc-windows-msvc.exe", "open-ontologies.exe"]
    }
    #[cfg(target_os = "macos")]
    {
        &[
            "open-ontologies-aarch64-apple-darwin",
            "open-ontologies-x86_64-apple-darwin",
            "open-ontologies",
        ]
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        &[
            "open-ontologies-x86_64-unknown-linux-gnu",
            "open-ontologies-x86_64-unknown-linux-musl",
            "open-ontologies",
        ]
    }
}

fn find_existing_binary(dir: &Path) -> Option<PathBuf> {
    engine_binary_names()
        .iter()
        .map(|name| dir.join(name))
        .find(|path| path.exists())
}

pub(crate) fn resolve_engine_binary(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(path) = exe.parent().and_then(find_existing_binary) {
            return Ok(path);
        }
    }
    if let Ok(dir) = app.path().resource_dir() {
        if let Some(path) = find_existing_binary(&dir) {
            return Ok(path);
        }
    }
    #[cfg(not(debug_assertions))]
    return Err("Bundled open-ontologies engine was not found".into());

    #[cfg(debug_assertions)]
    {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let bundled_dir = manifest_dir.join("binaries");
    if let Some(path) = find_existing_binary(&bundled_dir) {
        return Ok(path);
    }

    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .ok_or("Cannot resolve workspace root from Tauri manifest directory")?;
    for profile in ["release", "debug"] {
        let candidate_dir = workspace_root.join("target").join(profile);
        if let Some(path) = find_existing_binary(&candidate_dir) {
            return Ok(path);
        }
    }

    Err(format!(
        "No open-ontologies binary found. Checked {} and workspace target/{{release,debug}}.",
        bundled_dir.display()
    ))
    }
}

// Kept in sync by hand with the matching ENGINE_PORT constant in
// sidecars/agent/index.ts (`const ENGINE_PORT = process.env.OPEN_ONTOLOGIES_STUDIO_PORT ?? '8137'`).
// That copy is a fallback for running the sidecar standalone; in the normal
// path this Rust value always wins because spawn_agent_sidecar sets
// OPEN_ONTOLOGIES_STUDIO_PORT before the sidecar ever reads it, so the two
// never need to agree at runtime, only in source.
const DEFAULT_ENGINE_PORT: u16 = 8137;

fn parse_port(raw: Option<&str>) -> u16 {
    raw.and_then(|value| value.trim().parse::<u16>().ok())
        .filter(|port| *port != 0)
        .unwrap_or(DEFAULT_ENGINE_PORT)
}

/// Single source of truth for the engine's HTTP port. Every consumer of the
/// port (the REST base URL handed to the frontend, the MCP endpoint the
/// backend calls, and the environment variable passed to the agent sidecar)
/// derives its value from this function rather than resolving it again.
pub(crate) fn engine_port() -> u16 {
    let raw = std::env::var("OPEN_ONTOLOGIES_STUDIO_PORT").ok();
    parse_port(raw.as_deref())
}

/// Exposes the resolved engine port to the frontend so it does not need its
/// own copy of the port-resolution logic (and cannot drift from Rust's view
/// of it).
#[tauri::command]
pub fn get_engine_port() -> u16 {
    engine_port()
}

pub fn spawn_engine(app: &tauri::AppHandle) -> Result<(), String> {
    let binary = resolve_engine_binary(app)?;
    let port = engine_port();
    if std::net::TcpListener::bind(("127.0.0.1", port)).is_err() {
        return Err(format!("Engine port {port} is unavailable; set OPEN_ONTOLOGIES_STUDIO_PORT to a free port"));
    }

    eprintln!("[engine] spawning {}", binary.display());

    let mut child = Command::new(&binary)
        .args(["serve-http", "--port", &port.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn engine {}: {e}", binary.display()))?;

    let stderr = child.stderr.take().ok_or("No stderr")?;
    let app_handle = app.clone();
    std::thread::spawn(move || {
        let reader = std::io::BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(line) = line {
                eprintln!("[engine] {}", line);

            }
        }
        let _ = app_handle.emit("engine-stopped", true);
    });

    let state = app.state::<EngineState>();
    *state.child.lock().map_err(|e| format!("Lock error: {e}"))? = Some(child);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn port_defaults_to_8137_and_never_to_8080() {
        assert_eq!(parse_port(None), 8137);
        assert_eq!(parse_port(Some("9001")), 9001);
        assert_eq!(parse_port(Some("not-a-port")), 8137);
    }
}
