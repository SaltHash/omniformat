use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use chrono::SecondsFormat;
use tauri::{AppHandle, Emitter};

use crate::engine::run::TaskCtx;
use crate::engine::{self};
use crate::formats::{self, Family, Spec};
use crate::inspector;
use crate::settings;
use crate::tools::Tools;
use crate::types::{ConvertRequest, JobView, Options, Settings};

fn now_iso() -> String {
    chrono::Local::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

pub struct JobSpec {
    pub id: String,
    pub inputs: Vec<PathBuf>,
    pub output: PathBuf,
    pub target_id: String,
    pub source_family: Family,
    pub options: Options,
    pub tools: Tools,
    pub view: JobView,
}

struct Record {
    spec: JobSpec,
    cancel: Arc<AtomicBool>,
    view: JobView,
}

struct Inner {
    records: Vec<Record>,
    concurrency: usize,
}

pub struct JobManager {
    inner: Mutex<Inner>,
    app: AppHandle,
}

impl JobManager {
    pub fn new(app: AppHandle, concurrency: usize) -> Self {
        Self {
            inner: Mutex::new(Inner {
                records: Vec::new(),
                concurrency: concurrency.clamp(1, 6),
            }),
            app,
        }
    }

    fn emit(&self, view: &JobView) {
        let _ = self.app.emit("job:update", view.clone());
    }

    pub fn list(&self) -> Vec<JobView> {
        self.inner
            .lock()
            .map(|inner| inner.records.iter().map(|r| r.view.clone()).collect())
            .unwrap_or_default()
    }

    pub fn set_concurrency(&self, value: usize) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.concurrency = value.clamp(1, 6);
        }
    }

    pub fn enqueue(self: &Arc<Self>, specs: Vec<JobSpec>) -> Vec<JobView> {
        let views: Vec<JobView> = specs.iter().map(|s| s.view.clone()).collect();
        if let Ok(mut inner) = self.inner.lock() {
            for spec in specs {
                inner.records.push(Record {
                    cancel: Arc::new(AtomicBool::new(false)),
                    view: spec.view.clone(),
                    spec,
                });
            }
        }
        for view in &views {
            self.emit(view);
        }
        self.pump();
        views
    }

    pub fn cancel(self: &Arc<Self>, id: &str) {
        let mut update = None;
        let mut queued = false;
        if let Ok(mut inner) = self.inner.lock() {
            if let Some(record) = inner.records.iter_mut().find(|r| r.spec.id == id) {
                if record.view.status == "queued" {
                    record.view.status = "cancelled".into();
                    record.view.stage = "Cancelled".into();
                    record.view.finished_at = Some(now_iso());
                    queued = true;
                } else if record.view.status == "running" {
                    record.cancel.store(true, Ordering::Relaxed);
                    record.view.stage = "Cancelling".into();
                }
                update = Some(record.view.clone());
            }
        }
        if let Some(view) = update {
            self.emit(&view);
        }
        if queued {
            self.pump();
        }
    }

    pub fn clear_finished(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.records.retain(|r| {
                !matches!(r.view.status.as_str(), "done" | "error" | "cancelled")
            });
        }
    }

    pub fn pump(self: &Arc<Self>) {
        loop {
            let next = {
                let Ok(mut inner) = self.inner.lock() else {
                    return;
                };
                let running = inner
                    .records
                    .iter()
                    .filter(|r| r.view.status == "running")
                    .count();
                if running >= inner.concurrency {
                    return;
                }
                let Some(record) = inner
                    .records
                    .iter_mut()
                    .find(|r| r.view.status == "queued")
                else {
                    return;
                };
                record.view.status = "running".into();
                record.view.stage = "Preparing".into();
                record.view.progress = 0.5;
                (
                    record.view.clone(),
                    record.spec.clone_for_run(),
                    record.cancel.clone(),
                )
            };
            self.emit(&next.0);
            let manager = Arc::clone(self);
            std::thread::spawn(move || manager.run_job(next.1, next.2));
        }
    }

    fn set_progress(&self, id: &str, pct: f32, stage: &str) {
        let view = if let Ok(mut inner) = self.inner.lock() {
            if let Some(record) = inner.records.iter_mut().find(|r| r.spec.id == id) {
                record.view.progress = pct.clamp(0.0, 100.0) as f64;
                record.view.stage = stage.to_string();
                Some(record.view.clone())
            } else {
                None
            }
        } else {
            None
        };
        if let Some(view) = view {
            self.emit(&view);
        }
    }

    fn finish(
        self: &Arc<Self>,
        id: &str,
        status: &str,
        error: Option<String>,
        size_after: Option<u64>,
        output: Option<String>,
    ) {
        let view = {
            let Ok(mut inner) = self.inner.lock() else {
                return;
            };
            let Some(record) = inner.records.iter_mut().find(|r| r.spec.id == id) else {
                return;
            };
            record.view.status = status.to_string();
            record.view.finished_at = Some(now_iso());
            record.view.error = error.clone();
            record.view.size_after = size_after;
            if let Some(output) = output.as_ref() {
                record.view.output = output.clone();
                record.view.output_name = Path::new(output)
                    .file_name()
                    .map(|v| v.to_string_lossy().to_string())
                    .unwrap_or_else(|| output.clone());
            }
            if status == "done" {
                record.view.progress = 100.0;
                record.view.stage = "Done".into();
            } else if status == "cancelled" {
                record.view.stage = "Cancelled".into();
            } else {
                record.view.stage = "Failed".into();
            }
            let view = record.view.clone();

            Some(view)
        };
        if let Some(view) = view {
            self.emit(&view);
        }
        self.pump();
    }

    fn run_job(self: Arc<Self>, spec: JobSpec, cancel: Arc<AtomicBool>) {
        let parent = spec
            .output
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        if let Err(e) = std::fs::create_dir_all(&parent) {
            self.finish(&spec.id, "error", Some(format!("Could not create the output folder: {}", e)), None, None);
            return;
        }
        let temp = parent.join(format!(
            ".convert-{}-{}",
            &spec.id[..8.min(spec.id.len())],
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&temp);
        if let Err(e) = std::fs::create_dir_all(&temp) {
            self.finish(&spec.id, "error", Some(format!("Could not prepare the conversion: {}", e)), None, None);
            return;
        }
        let temp_output = temp.join(
            spec.output
                .file_name()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("output")),
        );

        let manager = Arc::clone(&self);
        let id = spec.id.clone();
        let progress: Arc<dyn Fn(f32, &str) + Send + Sync> = Arc::new(move |pct, stage| {
            manager.set_progress(&id, pct, stage);
        });

        let ctx = TaskCtx {
            tools: spec.tools.clone(),
            cancel: cancel.clone(),
            temp: temp.clone(),
            progress,
        };

        let input_refs: Vec<&Path> = spec.inputs.iter().map(|p| p.as_path()).collect();
        let result = engine::execute(
            &ctx,
            &input_refs,
            &temp_output,
            &spec.target_id,
            spec.source_family,
            &spec.options,
        );

        match result {
            Ok(()) if temp_output.exists() => {
                if spec.output.exists() {
                    let _ = std::fs::remove_file(&spec.output);
                }
                let moved = std::fs::rename(&temp_output, &spec.output).is_ok()
                    || std::fs::copy(&temp_output, &spec.output).is_ok();
                if !moved {
                    self.finish(
                        &spec.id,
                        "error",
                        Some("Could not move the finished file into place".into()),
                        None,
                        None,
                    );
                } else {
                    let size = std::fs::metadata(&spec.output).map(|m| m.len()).ok();
                    self.finish(
                        &spec.id,
                        "done",
                        None,
                        size,
                        Some(spec.output.to_string_lossy().to_string()),
                    );
                }
            }
            Ok(()) => {
                if cancel.load(Ordering::Relaxed) {
                    self.finish(&spec.id, "cancelled", None, None, None);
                } else {
                    self.finish(
                        &spec.id,
                        "error",
                        Some("The engine did not produce a file".into()),
                        None,
                        None,
                    );
                }
            }
            Err(error) => {
                if cancel.load(Ordering::Relaxed) || error == "Cancelled" {
                    self.finish(&spec.id, "cancelled", None, None, None);
                } else {
                    self.finish(&spec.id, "error", Some(error), None, None);
                }
            }
        }
        let _ = std::fs::remove_dir_all(&temp);
    }
}

impl JobSpec {
    fn clone_for_run(&self) -> JobSpec {
        JobSpec {
            id: self.id.clone(),
            inputs: self.inputs.clone(),
            output: self.output.clone(),
            target_id: self.target_id.clone(),
            source_family: self.source_family,
            options: self.options.clone(),
            tools: self.tools.clone(),
            view: self.view.clone(),
        }
    }
}

fn now_compact() -> (String, String) {
    let now = chrono::Local::now();
    (now.format("%Y-%m-%d").to_string(), now.format("%H%M%S").to_string())
}

fn sanitize_filename(value: &str) -> String {
    let cleaned: String = value
        .chars()
        .map(|c| if c == '/' || c == '\\' || c == '\0' { '-' } else { c })
        .collect();
    let cleaned = cleaned.trim().trim_matches('.').trim().to_string();
    if cleaned.is_empty() {
        "converted".into()
    } else if cleaned.chars().count() > 180 {
        cleaned.chars().take(180).collect()
    } else {
        cleaned
    }
}

pub fn target_extension(target: &Spec) -> String {
    target
        .extensions
        .first()
        .map(|s| s.to_string())
        .unwrap_or_else(|| target.id.to_string())
}

fn output_directory(
    input: &Path,
    settings: &Settings,
    request_dir: Option<&str>,
) -> PathBuf {
    if let Some(dir) = request_dir.map(str::trim).filter(|d| !d.is_empty()) {
        return PathBuf::from(dir);
    }
    if settings.output_mode == "custom" && !settings.output_dir.trim().is_empty() {
        return PathBuf::from(settings.output_dir.trim());
    }
    input
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn resolve_output(
    input: &Path,
    target: &Spec,
    settings: &Settings,
    request_dir: Option<&str>,
    source_family: Family,
    merged: bool,
    occupied: &HashSet<PathBuf>,
) -> Result<Option<PathBuf>, String> {
    let dir = output_directory(input, settings, request_dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Could not create {}: {}", dir.display(), e))?;

    let stem = input
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "converted".into());
    let ext = input
        .extension()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let (date, time) = now_compact();
    let template = if settings.filename_template.trim().is_empty() {
        "{name}"
    } else {
        settings.filename_template.trim()
    };
    let base = template
        .replace("{name}", &stem)
        .replace("{ext}", &ext)
        .replace("{target}", target.id)
        .replace("{date}", &date)
        .replace("{time}", &time);
    let suffix = engine::output_suffix(source_family, target, merged);
    let base = if let Some(suffix) = suffix {
        format!("{}-{}", base.trim_end_matches('-'), suffix)
    } else {
        base
    };
    let base = sanitize_filename(&base);
    let target_ext = target_extension(target);

    let mut candidate = dir.join(format!("{}.{}", base, target_ext));
    let taken = |p: &PathBuf| p.exists() || occupied.contains(p);
    if settings.overwrite == "skip" && taken(&candidate) {
        return Ok(None);
    }
    let overwrite = settings.overwrite == "overwrite";
    let same_as_input = candidate == input;
    let mut index = 1;
    while candidate.exists() || occupied.contains(&candidate) || candidate == input {
        if overwrite && !same_as_input && !occupied.contains(&candidate) {
            break;
        }
        candidate = dir.join(format!("{} ({}).{}", base, index, target_ext));
        index += 1;
    }
    Ok(Some(candidate))
}

pub fn build_specs(
    request: &ConvertRequest,
    tools: &Tools,
    settings: &Settings,
) -> Result<Vec<JobSpec>, String> {
    let target = formats::lookup(&request.target)
        .ok_or_else(|| format!("Unknown target format: {}", request.target))?;
    if request.inputs.is_empty() {
        return Err("Choose a file to convert".into());
    }

    let mut seen = HashSet::new();
    let mut inputs: Vec<(PathBuf, &'static Spec)> = Vec::new();
    for raw in &request.inputs {
        let path = PathBuf::from(raw);
        if !path.is_file() {
            return Err(format!("File not found: {}", raw));
        }
        if !seen.insert(path.clone()) {
            continue;
        }
        let spec = inspector::resolve(&path, tools)
            .ok_or_else(|| format!("This file type is not recognised: {}", path.display()))?;
        let targets = formats::targets_for(spec, tools);
        if !targets.iter().any(|t| t == &target.id) {
            return Err(format!(
                "{} cannot be converted to {}",
                spec.label, target.label
            ));
        }
        inputs.push((path, spec));
    }
    if inputs.is_empty() {
        return Err("Choose a file to convert".into());
    }

    let all_images = inputs
        .iter()
        .all(|(_, spec)| matches!(spec.family, Family::Image | Family::Vector));
    let merged_pdf = target.id == "pdf" && inputs.len() > 1 && all_images;
    let merged_video = matches!(target.family, Family::Video)
        && inputs.len() > 1
        && inputs.iter().all(|(_, spec)| spec.family == Family::Image);
    let merged = merged_pdf || merged_video;

    let mut specs = Vec::new();
    let mut occupied: HashSet<PathBuf> = HashSet::new();
    let created_at = now_iso();
    let mode = formats::mode_label(inputs[0].1.family, target);

    if merged {
        let first = &inputs[0];
        let output = resolve_output(
            &first.0,
            target,
            settings,
            request.output_dir.as_deref(),
            first.1.family,
            true,
            &occupied,
        )?
        .ok_or_else(|| "The output already exists and collisions are set to skip".to_string())?;
        occupied.insert(output.clone());
        let total_size = inputs
            .iter()
            .filter_map(|(p, _)| std::fs::metadata(p).ok().map(|m| m.len()))
            .sum();
        let id = uuid::Uuid::new_v4().to_string();
        let input_name = format!("{} images", inputs.len());
        let view = JobView {
            id: id.clone(),
            inputs: inputs.iter().map(|(p, _)| p.to_string_lossy().to_string()).collect(),
            input_name: input_name.clone(),
            input_count: inputs.len() as u32,
            output: output.to_string_lossy().to_string(),
            output_name: output
                .file_name()
                .map(|v| v.to_string_lossy().to_string())
                .unwrap_or_default(),
            target: target.id.into(),
            target_label: target.label.into(),
            target_family: target.family.id().into(),
            mode,
            status: "queued".into(),
            progress: 0.0,
            stage: "Queued".into(),
            error: None,
            size_before: total_size,
            size_after: None,
            created_at: created_at.clone(),
            finished_at: None,
        };
        specs.push(JobSpec {
            id,
            inputs: inputs.iter().map(|(p, _)| p.clone()).collect(),
            output,
            target_id: target.id.into(),
            source_family: first.1.family,
            options: request.options.clone(),
            tools: tools.clone(),
            view,
        });
    } else {
        for (path, source) in &inputs {
            let Some(output) = resolve_output(
                path,
                target,
                settings,
                request.output_dir.as_deref(),
                source.family,
                false,
                &occupied,
            )?
            else {
                continue;
            };
            occupied.insert(output.clone());
            let id = uuid::Uuid::new_v4().to_string();
            let name = path
                .file_name()
                .map(|v| v.to_string_lossy().to_string())
                .unwrap_or_else(|| path.to_string_lossy().to_string());
            let size_before = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
            let view = JobView {
                id: id.clone(),
                inputs: vec![path.to_string_lossy().to_string()],
                input_name: name,
                input_count: 1,
                output: output.to_string_lossy().to_string(),
                output_name: output
                    .file_name()
                    .map(|v| v.to_string_lossy().to_string())
                    .unwrap_or_default(),
                target: target.id.into(),
                target_label: target.label.into(),
                target_family: target.family.id().into(),
                mode: formats::mode_label(source.family, target),
                status: "queued".into(),
                progress: 0.0,
                stage: "Queued".into(),
                error: None,
                size_before,
                size_after: None,
                created_at: created_at.clone(),
                finished_at: None,
            };
            specs.push(JobSpec {
                id,
                inputs: vec![path.clone()],
                output,
                target_id: target.id.into(),
                source_family: source.family,
                options: request.options.clone(),
                tools: tools.clone(),
                view,
            });
        }
    }

    Ok(specs)
}
