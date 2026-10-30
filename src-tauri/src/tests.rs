use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::engine;
use crate::engine::run::TaskCtx;
use crate::formats::Family;
use crate::settings;
use crate::tools::Tools;
use crate::types::{Options, Settings};

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("convert-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("scratch directory");
        Scratch(dir)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn context(tools: &Tools, scratch: &Scratch) -> TaskCtx {
    TaskCtx {
        tools: tools.clone(),
        cancel: Arc::new(AtomicBool::new(false)),
        temp: scratch.path("tmp"),
        progress: Arc::new(|_: f32, _: &str| {}),
    }
}

fn make_still(tools: &Tools, path: &Path, color: &str) {
    let magick = tools.magick.as_deref().expect("ImageMagick");
    let status = Command::new(magick)
        .args(["-size", "160x100", &format!("xc:{}", color), &path.to_string_lossy()])
        .status()
        .expect("ImageMagick run");
    assert!(status.success(), "could not create test image");
}

#[test]
fn png_suggestions_are_a_short_ranked_shortlist() {
    let tools = Tools::detect();
    if !tools.has_images() {
        eprintln!("skipping: ImageMagick unavailable");
        return;
    }
    let source = crate::formats::lookup("png").expect("png format");
    let ranked = crate::formats::suggestions(source, &tools);
    let targets = crate::formats::targets_for(source, &tools);

    assert!(ranked.len() <= 5, "suggestions should stay short");
    assert!(!ranked.is_empty(), "png needs at least one suggestion");
    for expected in ["jpg", "webp", "ico"] {
        assert!(
            ranked.iter().any(|id| id == expected),
            "expected {expected} in {ranked:?}"
        );
    }
    for id in &ranked {
        assert_ne!(id, "png", "a source type is not its own suggestion");
        assert!(
            targets.contains(id),
            "suggested {id} is not a valid target: {targets:?}"
        );
    }
}

#[test]
fn default_targets_use_camel_case_and_survive_validation() {
    let mut settings = Settings::default();
    settings.default_targets.insert("png".into(), "ico".into());
    settings
        .default_targets
        .insert("missing-format".into(), "jpg".into());

    let json = serde_json::to_string(&settings).expect("serialize settings");
    assert!(json.contains("\"defaultTargets\""));
    let mut restored: Settings = serde_json::from_str(&json).expect("deserialize settings");
    assert_eq!(restored.default_targets.get("png").map(String::as_str), Some("ico"));

    settings::validate(&mut restored);
    assert_eq!(restored.default_targets.get("png").map(String::as_str), Some("ico"));
    assert!(!restored.default_targets.contains_key("missing-format"));
}

#[test]
fn merges_stills_into_a_multi_page_pdf() {
    let tools = Tools::detect();
    if tools.magick.is_none() {
        eprintln!("skipping: ImageMagick unavailable");
        return;
    }
    let scratch = Scratch::new();
    let first = scratch.path("first.png");
    let second = scratch.path("second.png");
    make_still(&tools, &first, "#3355ff");
    make_still(&tools, &second, "#ff7733");

    let output = scratch.path("merged.pdf");
    let ctx = context(&tools, &scratch);
    engine::execute(
        &ctx,
        &[first.as_path(), second.as_path()],
        &output,
        "pdf",
        Family::Image,
        &Options::default(),
    )
    .expect("merge images into PDF");

    assert!(output.metadata().map(|m| m.len() > 0).unwrap_or(false));
    if let Some(pdfinfo) = tools.pdfinfo.as_deref() {
        let text = Command::new(pdfinfo)
            .arg(&output)
            .output()
            .expect("pdfinfo run");
        let text = String::from_utf8_lossy(&text.stdout);
        let pages = text
            .lines()
            .find_map(|line| line.strip_prefix("Pages:"))
            .and_then(|value| value.trim().parse::<u32>().ok())
            .unwrap_or(0);
        assert_eq!(pages, 2, "expected one page per still");
    }
}

#[test]
fn merges_stills_into_a_slideshow_video() {
    let tools = Tools::detect();
    if tools.magick.is_none() || !tools.has_video() || !tools.has_video_encoder("h264") {
        eprintln!("skipping: image or video tooling unavailable");
        return;
    }
    let scratch = Scratch::new();
    let first = scratch.path("a.png");
    let second = scratch.path("b.png");
    make_still(&tools, &first, "#22c55e");
    make_still(&tools, &second, "#0b0b0c");

    let output = scratch.path("slideshow.mp4");
    let ctx = context(&tools, &scratch);
    engine::execute(
        &ctx,
        &[first.as_path(), second.as_path()],
        &output,
        "mp4",
        Family::Image,
        &Options {
            still_duration: Some(0.6),
            video_height: Some(240),
            ..Default::default()
        },
    )
    .expect("merge stills into video");
    assert!(output.metadata().map(|m| m.len() > 0).unwrap_or(false));

    if let Some(ffprobe) = tools.ffprobe.as_deref() {
        let out = Command::new(ffprobe)
            .args([
                "-v",
                "error",
                "-show_entries",
                "format=duration",
                "-of",
                "default=nw=1:nk=1",
                &output.to_string_lossy(),
            ])
            .output()
            .expect("ffprobe run");
        let seconds: f64 = String::from_utf8_lossy(&out.stdout)
            .trim()
            .parse()
            .unwrap_or(0.0);
        assert!(seconds >= 1.1, "slideshow was only {seconds}s long");
    }
}
