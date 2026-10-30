use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use serde_json::Value;

use crate::formats::{self, Family, Spec};
use crate::tools::Tools;
use crate::types::{FileMeta, FilmFrame};

fn base64_encode(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn data_url(mime: &str, path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.is_empty() {
        return None;
    }
    Some(format!("data:{};base64,{}", mime, base64_encode(&bytes)))
}

fn file_mime(tools: &Tools, path: &Path) -> Option<String> {
    let file = tools.file.as_deref()?;
    let out = Command::new(file)
        .args(["-b", "--mime-type", &path.to_string_lossy()])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn sniff(path: &Path) -> Option<&'static Spec> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).ok()?;
    let mut head = [0u8; 16];
    let n = f.read(&mut head).ok()?;
    let h = &head[..n];
    if h.starts_with(&[0x89, b'P', b'N', b'G']) {
        return formats::lookup("png");
    }
    if h.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return formats::lookup("jpg");
    }
    if h.starts_with(b"GIF87a") || h.starts_with(b"GIF89a") {
        return formats::lookup("gif");
    }
    if h.starts_with(b"BM") {
        return formats::lookup("bmp");
    }
    if h.starts_with(&[0x49, 0x49, 0x2A, 0x00]) || h.starts_with(&[0x4D, 0x4D, 0x00, 0x2A]) {
        return formats::lookup("tiff");
    }
    if h.len() >= 12 && &h[0..4] == b"RIFF" && &h[8..12] == b"WEBP" {
        return formats::lookup("webp");
    }
    if h.starts_with(b"8BPS") {
        return formats::lookup("psd");
    }
    if h.starts_with(b"qoif") {
        return formats::lookup("qoi");
    }
    if h.starts_with(&[0xFF, 0x0A]) || h.starts_with(&[0x00, 0x00, 0x00, 0x0C, 0x4A, 0x58, 0x4C, 0x20]) {
        return formats::lookup("jxl");
    }
    if h.starts_with(b"%PDF-") {
        return formats::lookup("pdf");
    }
    if h.starts_with(b"7z\xBC\xAF\x27\x1C") {
        return formats::lookup("7z");
    }
    if h.starts_with(b"Rar!\x1A\x07") {
        return formats::lookup("rar");
    }
    if h.starts_with(&[0x1F, 0x8B]) {
        return formats::lookup("tar.gz");
    }
    if h.starts_with(b"PK\x03\x04") {
        return formats::lookup("zip");
    }
    if h.starts_with(b"\x1A\x45\xDF\xA3") {
        return formats::lookup("mkv");
    }
    if h.starts_with(b"OggS") {
        return formats::lookup("ogg");
    }
    if h.starts_with(b"fLaC") {
        return formats::lookup("flac");
    }
    if h.starts_with(b"ID3") || (h.len() > 1 && h[0] == 0xFF && (h[1] & 0xE0) == 0xE0) {
        return formats::lookup("mp3");
    }
    if h.len() >= 12 && &h[4..8] == b"ftyp" {
        let brand = &h[8..12];
        if brand == b"qt  " {
            return formats::lookup("mov");
        }
        return formats::lookup("mp4");
    }
    None
}

pub fn resolve(path: &Path, tools: &Tools) -> Option<&'static Spec> {
    if let Some(spec) = formats::from_path(path) {
        return Some(spec);
    }
    if let Some(mime) = file_mime(tools, path) {
        if let Some(spec) = formats::from_mime(&mime) {
            return Some(spec);
        }
        if mime.starts_with("video/") {
            return formats::lookup("mp4");
        }
        if mime.starts_with("audio/") {
            return formats::lookup("mp3");
        }
        if mime == "image/svg+xml" {
            return formats::lookup("svg");
        }
        if mime.starts_with("image/") {
            return formats::lookup("png");
        }
        if mime == "application/pdf" {
            return formats::lookup("pdf");
        }
        if mime == "application/x-7z-compressed" {
            return formats::lookup("7z");
        }
    }
    sniff(path)
}

fn modified_ms(meta: &std::fs::Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn cache_key(path: &Path, meta: &std::fs::Metadata, suffix: &str) -> String {
    let mut hasher = DefaultHasher::new();
    path.to_string_lossy().hash(&mut hasher);
    meta.len().hash(&mut hasher);
    modified_ms(meta).hash(&mut hasher);
    suffix.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn magick_identify(tools: &Tools, path: &Path) -> Option<(u32, u32, u32)> {
    let magick = tools.magick.as_deref()?;
    let out = Command::new(magick)
        .args([
            "identify",
            "-format",
            "%w|%h|%n",
            &format!("{}[0]", path.to_string_lossy()),
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().next()?;
    let parts: Vec<&str> = line.split('|').collect();
    let w = parts.first()?.trim().parse().ok()?;
    let h = parts.get(1)?.trim().parse().ok()?;
    let n = parts.get(2).and_then(|v| v.trim().parse().ok()).unwrap_or(1);
    Some((w, h, n))
}

fn pdf_info(tools: &Tools, path: &Path) -> Option<(u32, Option<u32>, Option<u32>)> {
    let pdfinfo = tools.pdfinfo.as_deref()?;
    let out = Command::new(pdfinfo).arg(path).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut pages = 0u32;
    let mut width = None;
    let mut height = None;
    for line in text.lines() {
        if let Some(v) = line.strip_prefix("Pages:") {
            pages = v.trim().parse().unwrap_or(0);
        }
        if let Some(v) = line.strip_prefix("Page size:") {
            let cleaned = v.trim().split("pts").next().unwrap_or("");
            let parts: Vec<&str> = cleaned.split('x').collect();
            if parts.len() == 2 {
                width = parts[0].trim().parse::<f64>().ok().map(|v| v.round() as u32);
                height = parts[1].trim().parse::<f64>().ok().map(|v| v.round() as u32);
            }
        }
    }
    Some((pages, width, height))
}

#[derive(Default)]
struct MediaProbe {
    duration: Option<f64>,
    width: Option<u32>,
    height: Option<u32>,
    fps: Option<f64>,
    bitrate: Option<u64>,
    sample_rate: Option<u32>,
    channels: Option<u32>,
    video_codec: Option<String>,
    audio_codec: Option<String>,
    has_video: bool,
    has_audio: bool,
}

fn probe_media(tools: &Tools, path: &Path) -> MediaProbe {
    let mut probe = MediaProbe::default();
    let Some(ffprobe) = tools.ffprobe.as_deref() else {
        return probe;
    };
    let out = Command::new(ffprobe)
        .args([
            "-v",
            "error",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
            &path.to_string_lossy(),
        ])
        .output();
    let Ok(out) = out else { return probe };
    if !out.status.success() {
        return probe;
    }
    let Ok(json) = serde_json::from_slice::<Value>(&out.stdout) else {
        return probe;
    };
    if let Some(streams) = json["streams"].as_array() {
        for s in streams {
            match s["codec_type"].as_str().unwrap_or("") {
                "video" => {
                    if s["disposition"]["attached_pic"].as_u64().unwrap_or(0) == 0 {
                        probe.has_video = true;
                        probe.width = s["width"].as_u64().map(|v| v as u32);
                        probe.height = s["height"].as_u64().map(|v| v as u32);
                        probe.video_codec = s["codec_name"].as_str().map(|v| v.to_string());
                        if let Some(rate) = s["r_frame_rate"].as_str() {
                            let mut parts = rate.split('/');
                            if let (Some(num), Some(den)) = (parts.next(), parts.next()) {
                                if let (Ok(num), Ok(den)) = (num.parse::<f64>(), den.parse::<f64>()) {
                                    if den > 0.0 {
                                        probe.fps = Some((num / den * 100.0).round() / 100.0);
                                    }
                                }
                            }
                        }
                    }
                }
                "audio" => {
                    probe.has_audio = true;
                    probe.audio_codec = s["codec_name"].as_str().map(|v| v.to_string());
                    probe.sample_rate = s["sample_rate"].as_str().and_then(|v| v.parse().ok());
                    probe.channels = s["channels"].as_u64().map(|v| v as u32);
                }
                _ => {}
            }
        }
    }
    probe.duration = json["format"]["duration"]
        .as_str()
        .and_then(|v| v.parse::<f64>().ok());
    probe.bitrate = json["format"]["bit_rate"]
        .as_str()
        .and_then(|v| v.parse::<u64>().ok());
    probe
}

fn make_image_thumb(tools: &Tools, path: &Path, cache: &Path) -> Option<String> {
    let magick = tools.magick.as_deref()?;
    let dir = cache.join("previews");
    std::fs::create_dir_all(&dir).ok()?;
    let meta = std::fs::metadata(path).ok()?;
    let key = cache_key(path, &meta, "thumb");
    let out = dir.join(format!("{}.webp", key));
    if !out.exists() {
        let mut args: Vec<String> = Vec::new();
        let spec = formats::from_path(path);
        if spec.map(|s| matches!(s.family, Family::Vector | Family::Pdf)).unwrap_or(false) {
            args.push("-density".into());
            args.push("110".into());
        }
        args.push(format!("{}[0]", path.to_string_lossy()));
        args.extend([
            "-auto-orient".into(),
            "-thumbnail".into(),
            "900x560>".into(),
            "-strip".into(),
            "-quality".into(),
            "82".into(),
            out.to_string_lossy().to_string(),
        ]);
        let status = Command::new(magick).args(&args).status().ok()?;
        if !status.success() || !out.exists() {
            return None;
        }
    }
    data_url("image/webp", &out)
}

fn make_video_thumb(tools: &Tools, path: &Path, cache: &Path, duration: Option<f64>) -> Option<String> {
    let ffmpeg = tools.ffmpeg.as_deref()?;
    let dir = cache.join("previews");
    std::fs::create_dir_all(&dir).ok()?;
    let meta = std::fs::metadata(path).ok()?;
    let key = cache_key(path, &meta, "vthumb");
    let out = dir.join(format!("{}.jpg", key));
    if !out.exists() {
        let time = duration.map(|d| (d * 0.12).min(2.0)).unwrap_or(0.0);
        let args = vec![
            "-hide_banner".to_string(),
            "-loglevel".into(),
            "error".into(),
            "-nostdin".into(),
            "-y".into(),
            "-ss".into(),
            format!("{:.3}", time),
            "-i".into(),
            path.to_string_lossy().to_string(),
            "-frames:v".into(),
            "1".into(),
            "-vf".into(),
            "scale=900:-2".into(),
            "-q:v".into(),
            "3".into(),
            out.to_string_lossy().to_string(),
        ];
        let status = Command::new(ffmpeg).args(&args).status().ok()?;
        if !status.success() || !out.exists() {
            return None;
        }
    }
    data_url("image/jpeg", &out)
}

fn make_audio_thumb(tools: &Tools, path: &Path, cache: &Path) -> Option<String> {
    let ffmpeg = tools.ffmpeg.as_deref()?;
    let dir = cache.join("previews");
    std::fs::create_dir_all(&dir).ok()?;
    let meta = std::fs::metadata(path).ok()?;
    let key = cache_key(path, &meta, "athumb");
    let out = dir.join(format!("{}.png", key));
    if !out.exists() {
        let args = vec![
            "-hide_banner".to_string(),
            "-loglevel".into(),
            "error".into(),
            "-nostdin".into(),
            "-y".into(),
            "-t".into(),
            "120".into(),
            "-i".into(),
            path.to_string_lossy().to_string(),
            "-filter_complex".into(),
            "aformat=channel_layouts=mono,showwavespic=s=900x260:colors=0xF0A35E".into(),
            "-frames:v".into(),
            "1".into(),
            out.to_string_lossy().to_string(),
        ];
        let status = Command::new(ffmpeg).args(&args).status().ok()?;
        if !status.success() || !out.exists() {
            return None;
        }
    }
    data_url("image/png", &out)
}

pub fn inspect(path: &Path, tools: &Tools, cache: &Path) -> FileMeta {
    let mut meta = FileMeta {
        path: path.to_string_lossy().to_string(),
        name: path
            .file_name()
            .map(|v| v.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string()),
        preview_kind: "none".into(),
        ..Default::default()
    };
    let Ok(fs_meta) = std::fs::metadata(path) else {
        meta.error = Some("File not found".into());
        meta.format_label = "Missing".into();
        meta.family = "other".into();
        return meta;
    };
    if fs_meta.is_dir() {
        meta.error = Some("Folders are not converted directly".into());
        meta.format_label = "Folder".into();
        meta.family = "other".into();
        return meta;
    }
    meta.size = fs_meta.len();
    meta.modified = modified_ms(&fs_meta);

    let spec = resolve(path, tools);
    if let Some(spec) = spec {
        meta.format_id = spec.id.into();
        meta.format_label = spec.label.into();
        meta.family = spec.family.id().into();
        meta.mime = spec.mime.into();
    } else {
        meta.format_id = "unknown".into();
        meta.format_label = path
            .extension()
            .map(|e| e.to_string_lossy().to_uppercase())
            .filter(|e| !e.is_empty())
            .unwrap_or_else(|| "File".into());
        meta.family = "other".into();
        meta.mime = file_mime(tools, path).unwrap_or_else(|| "application/octet-stream".into());
        meta.preview_kind = "unknown".into();
        meta.error = Some("This type is not recognised".into());
        return meta;
    }

    let spec = spec.unwrap();
    meta.extension = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    match spec.family {
        Family::Image | Family::Vector => {
            meta.preview_kind = "image".into();
            if let Some((w, h, pages)) = magick_identify(tools, path) {
                meta.width = Some(w);
                meta.height = Some(h);
                if pages > 1 {
                    meta.pages = Some(pages);
                }
            }
            meta.thumbnail = make_image_thumb(tools, path, cache);
        }
        Family::Pdf => {
            meta.preview_kind = "pdf".into();
            if let Some((pages, w, h)) = pdf_info(tools, path) {
                if pages > 0 {
                    meta.pages = Some(pages);
                }
                meta.width = w;
                meta.height = h;
            }
            meta.thumbnail = make_image_thumb(tools, path, cache);
        }
        Family::Video | Family::Audio => {
            let probe = probe_media(tools, path);
            meta.duration = probe.duration;
            meta.width = probe.width;
            meta.height = probe.height;
            meta.fps = probe.fps;
            meta.bitrate = probe.bitrate;
            meta.sample_rate = probe.sample_rate;
            meta.channels = probe.channels;
            meta.video_codec = probe.video_codec;
            meta.audio_codec = probe.audio_codec;
            meta.has_video = probe.has_video;
            meta.has_audio = probe.has_audio;
            if spec.family == Family::Video {
                meta.preview_kind = "video".into();
                meta.thumbnail = make_video_thumb(tools, path, cache, probe.duration);
            } else {
                meta.preview_kind = "audio".into();
                meta.thumbnail = make_audio_thumb(tools, path, cache);
            }
        }
        Family::Document | Family::Spreadsheet | Family::Presentation => {
            meta.preview_kind = "document".into();
        }
        Family::Archive => {
            meta.preview_kind = "archive".into();
        }
        Family::Subtitle => {
            meta.preview_kind = "subtitle".into();
        }
    }

    meta
}

pub fn filmstrip(path: &Path, tools: &Tools, cache: &Path) -> Vec<FilmFrame> {
    let Some(ffmpeg) = tools.ffmpeg.as_deref() else {
        return Vec::new();
    };
    let probe = probe_media(tools, path);
    let Some(duration) = probe.duration.filter(|d| *d > 0.05) else {
        return Vec::new();
    };
    let count = if duration < 3.0 { 6 } else { 12 };
    let fps = (count as f64 / duration).min(30.0);
    let dir = cache.join("filmstrips");
    let _ = std::fs::create_dir_all(&dir);
    let temp = dir.join(format!(
        "{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    let _ = std::fs::create_dir_all(&temp);
    let pattern = temp.join("%02d.jpg");

    let args = vec![
        "-hide_banner".to_string(),
        "-loglevel".into(),
        "error".into(),
        "-nostdin".into(),
        "-y".into(),
        "-i".into(),
        path.to_string_lossy().to_string(),
        "-vf".into(),
        format!("fps={:.6},scale=240:-2", fps),
        "-frames:v".into(),
        count.to_string(),
        "-q:v".into(),
        "4".into(),
        pattern.to_string_lossy().to_string(),
    ];
    let status = Command::new(ffmpeg).args(&args).status();
    let mut frames = Vec::new();
    if status.map(|s| s.success()).unwrap_or(false) {
        let mut files: Vec<PathBuf> = std::fs::read_dir(&temp)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .filter(|p| p.extension().map(|x| x == "jpg").unwrap_or(false))
                    .collect()
            })
            .unwrap_or_default();
        files.sort();
        for (i, file) in files.iter().enumerate() {
            if let Some(image) = data_url("image/jpeg", file) {
                frames.push(FilmFrame {
                    time: (i as f64 * duration / files.len().max(1) as f64 * 100.0).round() / 100.0,
                    image,
                });
            }
        }
    }
    let _ = std::fs::remove_dir_all(&temp);
    frames
}

pub fn frame_at(path: &Path, tools: &Tools, time: f64) -> Result<String, String> {
    let ffmpeg = tools.ffmpeg.as_deref().ok_or("FFmpeg is not available")?;
    let dir = std::env::temp_dir().join("convert-frames");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let out = dir.join(format!("frame-{}.jpg", uuid::Uuid::new_v4()));
    let args = vec![
        "-hide_banner".to_string(),
        "-loglevel".into(),
        "error".into(),
        "-nostdin".into(),
        "-y".into(),
        "-ss".into(),
        format!("{:.3}", time.max(0.0)),
        "-i".into(),
        path.to_string_lossy().to_string(),
        "-frames:v".into(),
        "1".into(),
        "-vf".into(),
        "scale=960:-2".into(),
        "-q:v".into(),
        "3".into(),
        out.to_string_lossy().to_string(),
    ];
    let status = Command::new(ffmpeg).args(&args).status().map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("Could not render this frame".into());
    }
    let url = data_url("image/jpeg", &out).ok_or("Could not read the frame")?;
    let _ = std::fs::remove_file(&out);
    Ok(url)
}

fn uris_to_paths(text: &str) -> Vec<String> {
    let mut paths = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line == "copy" || line == "cut" {
            continue;
        }
        let raw = if let Some(rest) = line.strip_prefix("file://") {
            let decoded = rest
                .replace("%20", " ")
                .replace("%23", "#")
                .replace("%25", "%")
                .replace("%5B", "[")
                .replace("%5D", "]");
            decoded
        } else {
            line.to_string()
        };
        let path = PathBuf::from(raw);
        if path.is_file() {
            paths.push(path.to_string_lossy().to_string());
        }
    }
    paths
}

fn wl_paste(tools: &Tools, args: &[&str]) -> Option<String> {
    let wl = tools.wl_paste.as_deref()?;
    let out = Command::new(wl).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).to_string())
}

pub fn clipboard_files(tools: &Tools, cache: &Path) -> Vec<String> {
    if let Some(types) = wl_paste(tools, &["--list-types"]) {
        if types.contains("text/uri-list") {
            if let Some(text) = wl_paste(tools, &["--no-newline", "--type", "text/uri-list"]) {
                let paths = uris_to_paths(&text);
                if !paths.is_empty() {
                    return paths;
                }
            }
        }
        if types.contains("x-special/gnome-copied-files") {
            if let Some(text) = wl_paste(tools, &["--no-newline", "--type", "x-special/gnome-copied-files"]) {
                let paths = uris_to_paths(&text);
                if !paths.is_empty() {
                    return paths;
                }
            }
        }
        for mime in ["image/png", "image/jpeg", "image/webp"] {
            if types.contains(mime) {
                let wl = tools.wl_paste.as_deref().unwrap();
                if let Ok(out) = Command::new(wl).args(["--type", mime]).output() {
                    if out.status.success() && !out.stdout.is_empty() {
                        let ext = match mime {
                            "image/png" => "png",
                            "image/jpeg" => "jpg",
                            _ => "webp",
                        };
                        let dir = cache.join("clipboard");
                        let _ = std::fs::create_dir_all(&dir);
                        let path = dir.join(format!("clipboard-{}.{}", uuid::Uuid::new_v4(), ext));
                        if std::fs::write(&path, &out.stdout).is_ok() {
                            return vec![path.to_string_lossy().to_string()];
                        }
                    }
                }
            }
        }
        if let Some(text) = wl_paste(tools, &["--no-newline", "--type", "text/plain"]) {
            let trimmed = text.trim();
            let path = if let Some(rest) = trimmed.strip_prefix("file://") {
                PathBuf::from(rest.replace("%20", " "))
            } else {
                PathBuf::from(trimmed)
            };
            if path.is_file() {
                return vec![path.to_string_lossy().to_string()];
            }
        }
    }

    if let Some(xclip) = tools.xclip.as_deref() {
        if let Ok(out) = Command::new(xclip)
            .args(["-selection", "clipboard", "-t", "text/uri-list", "-o"])
            .output()
        {
            let paths = uris_to_paths(&String::from_utf8_lossy(&out.stdout));
            if !paths.is_empty() {
                return paths;
            }
        }
        if let Ok(out) = Command::new(xclip)
            .args(["-selection", "clipboard", "-t", "text/plain", "-o"])
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            if let Some(path) = uris_to_paths(&text).first() {
                return vec![path.clone()];
            }
        }
    }

    Vec::new()
}

pub fn save_temp_image(bytes: &[u8], mime: &str, cache: &Path) -> Result<String, String> {
    let ext = match mime {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        "image/gif" => "gif",
        _ => return Err("Unsupported clipboard image type".into()),
    };
    if bytes.is_empty() {
        return Err("The clipboard image is empty".into());
    }
    if bytes.len() > 64 * 1024 * 1024 {
        return Err("The clipboard image is too large".into());
    }
    let dir = cache.join("clipboard");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("pasted-{}.{}", uuid::Uuid::new_v4(), ext));
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

pub fn cache_root() -> PathBuf {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("HOME")
                .map(|h| PathBuf::from(h).join(".cache"))
                .unwrap_or_else(std::env::temp_dir)
        });
    base.join("convert")
}

#[allow(dead_code)]
pub fn file_size(path: &Path) -> u64 {
    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

#[allow(dead_code)]
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
