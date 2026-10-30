use serde::{Deserialize, Serialize};

/// User-facing conversion options. All fields are optional so the frontend can
/// send only the knobs that matter for the current route.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Options {
    /// Perceptual quality, 1-100. Higher is better/bigger.
    pub quality: Option<u8>,
    /// Longest edge in pixels. 0 or None keeps the original size.
    pub max_dimension: Option<u32>,
    /// Target video height (720, 1080, 1440, 2160). 0 keeps the source.
    pub video_height: Option<u32>,
    pub fps: Option<u32>,
    pub trim_start: Option<f64>,
    pub trim_end: Option<f64>,
    /// Frame to extract from a video, in seconds.
    pub frame_time: Option<f64>,
    pub audio_bitrate: Option<u32>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u8>,
    pub normalize_audio: Option<bool>,
    pub mute: Option<bool>,
    /// Strip metadata where the tool allows it.
    pub strip_metadata: Option<bool>,
    /// Page number for PDF rasterisation, 1-based.
    pub page: Option<u32>,
    pub dpi: Option<u32>,
    /// Compression level for archives, 1-9.
    pub compression: Option<u8>,
    /// Seconds each still image is shown when building a video.
    pub still_duration: Option<f64>,
    /// Preserve subtitles when possible.
    pub keep_subtitles: Option<bool>,
}

impl Options {
    pub fn quality_or(&self, fallback: u8) -> u8 {
        self.quality.unwrap_or(fallback).clamp(1, 100)
    }
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FormatInfo {
    pub id: String,
    pub label: String,
    pub family: String,
    pub extensions: Vec<String>,
    pub mime: String,
    pub note: Option<String>,
    pub lossless: bool,
    pub can_read: bool,
    pub can_write: bool,
    pub targets: Vec<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ToolInfo {
    pub id: String,
    pub name: String,
    pub role: String,
    pub available: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub note: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub formats: Vec<FormatInfo>,
    pub tools: Vec<ToolInfo>,
    /// Ranked shortlist of targets for each readable format id (at most five).
    pub suggestions: std::collections::HashMap<String, Vec<String>>,
    pub format_count: usize,
    pub available_count: usize,
    /// Which cross-format behaviours this machine can perform.
    pub cross: Vec<String>,
    pub app_version: String,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct FileMeta {
    pub path: String,
    pub name: String,
    pub extension: String,
    pub format_id: String,
    pub format_label: String,
    pub family: String,
    pub mime: String,
    pub size: u64,
    pub modified: i64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration: Option<f64>,
    pub fps: Option<f64>,
    pub bitrate: Option<u64>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
    pub pages: Option<u32>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub has_video: bool,
    pub has_audio: bool,
    pub preview_kind: String,
    pub detail: Option<String>,
    pub thumbnail: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FilmFrame {
    pub time: f64,
    pub image: String,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Waveform {
    pub image: String,
    pub duration: f64,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct JobView {
    pub id: String,
    pub inputs: Vec<String>,
    pub input_name: String,
    pub input_count: u32,
    pub output: String,
    pub output_name: String,
    pub target: String,
    pub target_label: String,
    pub target_family: String,
    pub mode: String,
    pub status: String,
    pub progress: f64,
    pub stage: String,
    pub error: Option<String>,
    pub size_before: u64,
    pub size_after: Option<u64>,
    pub created_at: String,
    pub finished_at: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// "source" keeps outputs next to their input, "custom" uses output_dir.
    pub output_mode: String,
    pub output_dir: String,
    pub filename_template: String,
    /// "rename" | "overwrite" | "skip"
    pub overwrite: String,
    pub concurrency: u8,
    /// "dark" | "light" | "system"
    pub theme: String,
    pub default_quality: u8,
    pub keep_metadata: bool,
    pub reduce_motion: bool,
    /// Reveal the output in the file manager when a job finishes.
    pub reveal_on_finish: bool,
    /// Preferred output format for each source format id. Empty means the
    /// catalog's top suggestion.
    pub default_targets: std::collections::HashMap<String, String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            output_mode: "source".into(),
            output_dir: String::new(),
            filename_template: "{name}".into(),
            overwrite: "rename".into(),
            concurrency: 2,
            theme: "dark".into(),
            default_quality: 85,
            keep_metadata: true,
            reduce_motion: false,
            reveal_on_finish: true,
            default_targets: std::collections::HashMap::new(),
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ConvertRequest {
    pub inputs: Vec<String>,
    pub target: String,
    #[serde(default)]
    pub options: Options,
    pub output_dir: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StartResponse {
    pub jobs: Vec<JobView>,
}
