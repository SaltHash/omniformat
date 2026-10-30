use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use base64::Engine;
use tauri::{AppHandle, Manager, State};

use crate::formats;
use crate::inspector;
use crate::jobs::{self, JobManager};
use crate::settings;
use crate::tools::Tools;
use crate::types::{
    Catalog, ConvertRequest, FileMeta, FilmFrame, JobView, Settings, StartResponse,
};

pub struct AppState {
    pub tools: Mutex<Tools>,
    pub settings: Mutex<Settings>,
    pub jobs: Arc<JobManager>,
}

/// Lets the webview play a file the user selected through Tauri's asset protocol.
fn allow_preview(app: &AppHandle, path: &str) {
    let _ = app.asset_protocol_scope().allow_file(path);
}

#[tauri::command]
pub fn get_catalog(state: State<'_, AppState>) -> Catalog {
    let tools = state.tools.lock().unwrap();
    formats::catalog(&tools)
}

#[tauri::command]
pub fn refresh_catalog(state: State<'_, AppState>) -> Catalog {
    let tools = Tools::detect();
    let catalog = formats::catalog(&tools);
    *state.tools.lock().unwrap() = tools;
    catalog
}

#[tauri::command]
pub fn inspect_paths(
    paths: Vec<String>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Vec<FileMeta> {
    for path in &paths {
        allow_preview(&app, path);
    }
    let tools = state.tools.lock().unwrap().clone();
    let cache = settings::cache_dir();
    if paths.is_empty() {
        return Vec::new();
    }
    let next = AtomicUsize::new(0);
    let mut results: Vec<Option<FileMeta>> = (0..paths.len()).map(|_| None).collect();
    let workers = paths.len().min(4).max(1);

    std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for _ in 0..workers {
            let tools = &tools;
            let cache = &cache;
            let paths = &paths;
            let next = &next;
            handles.push(scope.spawn(move || {
                let mut out = Vec::new();
                loop {
                    let index = next.fetch_add(1, Ordering::SeqCst);
                    if index >= paths.len() {
                        break;
                    }
                    let meta = inspector::inspect(Path::new(&paths[index]), tools, cache);
                    out.push((index, meta));
                }
                out
            }));
        }
        for handle in handles {
            if let Ok(items) = handle.join() {
                for (index, meta) in items {
                    results[index] = Some(meta);
                }
            }
        }
    });

    results
        .into_iter()
        .map(|item| item.unwrap_or_default())
        .collect()
}

#[tauri::command]
pub fn inspect_path(path: String, app: AppHandle, state: State<'_, AppState>) -> FileMeta {
    allow_preview(&app, &path);
    let tools = state.tools.lock().unwrap();
    inspector::inspect(Path::new(&path), &tools, &settings::cache_dir())
}

#[tauri::command]
pub fn filmstrip(path: String, state: State<'_, AppState>) -> Vec<FilmFrame> {
    let tools = state.tools.lock().unwrap();
    inspector::filmstrip(Path::new(&path), &tools, &settings::cache_dir())
}

#[tauri::command]
pub fn frame_at(path: String, time: f64, state: State<'_, AppState>) -> Result<String, String> {
    let tools = state.tools.lock().unwrap();
    inspector::frame_at(Path::new(&path), &tools, time)
}

#[tauri::command]
pub fn clipboard_files(state: State<'_, AppState>) -> Vec<String> {
    let tools = state.tools.lock().unwrap();
    inspector::clipboard_files(&tools, &settings::cache_dir())
}

#[tauri::command]
pub fn save_temp_image(data: String, mime: String) -> Result<String, String> {
    if data.len() > 90 * 1024 * 1024 {
        return Err("That image is too large".into());
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data.as_bytes())
        .map_err(|_| "Could not read the pasted image".to_string())?;
    inspector::save_temp_image(&bytes, &mime, &settings::cache_dir())
}

#[tauri::command]
pub fn start_conversion(
    request: ConvertRequest,
    state: State<'_, AppState>,
) -> Result<StartResponse, String> {
    let tools = state.tools.lock().unwrap().clone();
    let settings = state.settings.lock().unwrap().clone();
    let specs = jobs::build_specs(&request, &tools, &settings)?;
    let views = state.jobs.enqueue(specs);
    Ok(StartResponse { jobs: views })
}

#[tauri::command]
pub fn list_jobs(state: State<'_, AppState>) -> Vec<JobView> {
    state.jobs.list()
}

#[tauri::command]
pub fn cancel_job(id: String, state: State<'_, AppState>) {
    state.jobs.cancel(&id);
}

#[tauri::command]
pub fn clear_jobs(state: State<'_, AppState>) {
    state.jobs.clear_finished();
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
pub fn set_settings(settings: Settings, state: State<'_, AppState>) -> Result<Settings, String> {
    let mut next = settings;
    crate::settings::validate(&mut next);
    crate::settings::save_settings(&next)?;
    state.jobs.set_concurrency(next.concurrency as usize);
    *state.settings.lock().unwrap() = next.clone();
    state.jobs.pump();
    Ok(next)
}

#[tauri::command]
pub fn clear_cache() -> Result<(), String> {
    settings::clear_cache()
}

#[tauri::command]
pub fn open_output(path: String, reveal: bool) -> Result<(), String> {
    let target = PathBuf::from(&path);
    let target = if reveal {
        target
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from(&path))
    } else {
        target
    };
    if !target.exists() {
        return Err("That file is no longer there".into());
    }
    let candidates = ["xdg-open", "gio"];
    for program in candidates {
        let mut command = std::process::Command::new(program);
        if program == "gio" {
            command.arg("open");
        }
        command.arg(&target);
        if let Ok(mut child) = command.spawn() {
            std::thread::spawn(move || {
                let _ = child.wait();
            });
            return Ok(());
        }
    }
    Err("Could not open the file manager".into())
}

#[tauri::command]
pub fn config_location() -> String {
    settings::config_dir().to_string_lossy().to_string()
}
