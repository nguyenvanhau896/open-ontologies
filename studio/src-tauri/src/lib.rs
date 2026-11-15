// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

mod chat;
mod corpus;
mod engine;
mod mcp;

use chat::ChatState;
use corpus::CorpusState;
use engine::EngineState;
use mcp::McpState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::Manager;

#[tauri::command]
fn engine_status(state: tauri::State<EngineState>) -> bool {
    state.running.load(Ordering::Relaxed)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(EngineState {
            running: Arc::new(AtomicBool::new(false)),
            child: Mutex::new(None),
        })
        .manage(ChatState {
            process: Mutex::new(None),
        })
        .manage(McpState {
            session_id: Mutex::new(None),
            client: reqwest::Client::new(),
        })
        .manage(CorpusState {
            running: Arc::new(Mutex::new(false)),
        })
        .setup(|app| {
            let workspace = app.path().app_data_dir()?.join("workspace");
            std::fs::create_dir_all(workspace.join("demo/derived"))?;
            std::env::set_var("ONTO_STUDIO_WORKSPACE", &workspace);
            let handle = app.handle().clone();
            engine::spawn_engine(&handle).map_err(std::io::Error::other)?;
            let agent_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let client = reqwest::Client::new();
                for _ in 0..60 {
                    let ready = client.get(format!("http://127.0.0.1:{}/api/stats", engine::engine_port()))
                        .timeout(std::time::Duration::from_secs(1)).send().await
                        .is_ok_and(|r| r.status().is_success());
                    if ready {
                        agent_handle.state::<EngineState>().running.store(true, Ordering::Relaxed);
                        let _ = tauri::Emitter::emit(&agent_handle, "engine-ready", true);
                        if let Err(e) = chat::spawn_agent_sidecar(&agent_handle) { eprintln!("Failed to start agent: {e}"); }
                        return;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
                let _ = tauri::Emitter::emit(&agent_handle, "engine-stopped", true);
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            engine_status,
            engine::get_engine_port,
            mcp::mcp_call,
            mcp::set_mcp_session,
            chat::send_chat_message,
            chat::reset_chat,
            corpus::corpus_presets,
            corpus::ingest_corpus,
            corpus::read_store,
            corpus::list_graphs,
            corpus::read_decisions,
            corpus::revert_type,
            corpus::list_saved,
            corpus::pick_ontology_file
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                // Kill agent sidecar
                if let Some(mut child) = app_handle
                    .state::<ChatState>()
                    .process
                    .lock()
                    .ok()
                    .and_then(|mut g| g.take())
                {
                    let _ = child.kill();
                }
                // Kill engine
                if let Some(mut child) = app_handle
                    .state::<EngineState>()
                    .child
                    .lock()
                    .ok()
                    .and_then(|mut g| g.take())
                {
                    let _ = child.kill();
                }
            }
        });
}
