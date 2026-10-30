#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod engine;
mod formats;
mod inspector;
mod jobs;
mod settings;
mod tools;
mod types;

#[cfg(test)]
mod tests;

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use tauri::Manager;

use commands::AppState;
use engine::run::TaskCtx;
use jobs::JobManager;
use tools::Tools;
use types::Options;

fn cli() -> bool {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        return false;
    }
    match args[1].as_str() {
        "--catalog" => {
            let tools = Tools::detect();
            match serde_json::to_string_pretty(&formats::catalog(&tools)) {
                Ok(text) => println!("{}", text),
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            }
            true
        }
        "--inspect" => {
            let Some(path) = args.get(2) else {
                eprintln!("usage: omniformat --inspect <file>");
                std::process::exit(2);
            };
            let tools = Tools::detect();
            let meta = inspector::inspect(
                std::path::Path::new(path),
                &tools,
                &settings::cache_dir(),
            );
            match serde_json::to_string_pretty(&meta) {
                Ok(text) => println!("{}", text),
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            }
            true
        }
        "--convert" => {
            if args.len() < 5 {
                eprintln!("usage: omniformat --convert <input> <target> <output> [options-json]");
                std::process::exit(2);
            }
            let input = PathBuf::from(&args[2]);
            let target_id = &args[3];
            let output = PathBuf::from(&args[4]);
            let options: Options = args
                .get(5)
                .and_then(|raw| serde_json::from_str(raw).ok())
                .unwrap_or_default();
            let tools = Tools::detect();
            let Some(source) = inspector::resolve(&input, &tools) else {
                eprintln!("error: unrecognised input");
                std::process::exit(1);
            };
            if formats::lookup(target_id).is_none() {
                eprintln!("error: unknown target {}", target_id);
                std::process::exit(1);
            }
            if let Some(parent) = output.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let temp = std::env::temp_dir().join(format!("convert-cli-{}", std::process::id()));
            let _ = std::fs::create_dir_all(&temp);
            let ctx = TaskCtx {
                tools: tools.clone(),
                cancel: Arc::new(AtomicBool::new(false)),
                temp: temp.clone(),
                progress: Arc::new(|pct: f32, stage: &str| {
                    eprintln!("{:>5.1}%  {}", pct, stage);
                }),
            };
            let result = engine::execute(
                &ctx,
                &[input.as_path()],
                &output,
                target_id,
                source.family,
                &options,
            );
            let _ = std::fs::remove_dir_all(&temp);
            match result {
                Ok(()) if output.exists() => {
                    println!("{}", output.to_string_lossy());
                }
                Ok(()) => {
                    eprintln!("error: no output produced");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("error: {}", e);
                    std::process::exit(1);
                }
            }
            true
        }
        _ => false,
    }
}

fn run_app() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let settings = settings::load_settings();
            let tools = Tools::detect();
            let jobs = Arc::new(JobManager::new(
                app.handle().clone(),
                settings.concurrency as usize,
            ));
            app.manage(AppState {
                tools: Mutex::new(tools),
                settings: Mutex::new(settings),
                jobs,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_catalog,
            commands::refresh_catalog,
            commands::inspect_paths,
            commands::inspect_path,
            commands::filmstrip,
            commands::frame_at,
            commands::clipboard_files,
            commands::save_temp_image,
            commands::start_conversion,
            commands::list_jobs,
            commands::cancel_job,
            commands::clear_jobs,
            commands::get_settings,
            commands::set_settings,
            commands::clear_cache,
            commands::open_output,
            commands::config_location,
        ])
        .run(tauri::generate_context!())
        .expect("error while running omniformat");
}

fn main() {
    if cli() {
        return;
    }
    run_app();
}
