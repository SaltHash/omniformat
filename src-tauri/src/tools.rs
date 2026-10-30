use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::types::ToolInfo;

#[derive(Clone, Debug, Default)]
pub struct Tools {
    pub magick: Option<String>,
    pub magick_version: String,
    pub magick_formats: HashMap<String, (bool, bool, String)>,
    pub ffmpeg: Option<String>,
    pub ffmpeg_version: String,
    pub ffprobe: Option<String>,
    pub encoders: HashSet<String>,
    pub muxers: HashSet<String>,
    pub demuxers: HashSet<String>,
    pub soffice: Option<String>,
    pub soffice_version: String,
    pub gs: Option<String>,
    pub gs_version: String,
    pub pdfinfo: Option<String>,
    pub pdftoppm: Option<String>,
    pub pdftotext: Option<String>,
    pub pdftocairo: Option<String>,
    pub sevenzip: Option<String>,
    pub sevenzip_version: String,
    pub sevenzip_formats: HashSet<String>,
    pub tar: Option<String>,
    pub zstd: Option<String>,
    pub gzip: Option<String>,
    pub bzip2: Option<String>,
    pub xz: Option<String>,
    pub file: Option<String>,
    pub wl_paste: Option<String>,
    pub xclip: Option<String>,
}

fn which(name: &str) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(meta) = candidate.metadata() {
                    if meta.permissions().mode() & 0o111 != 0 {
                        return Some(candidate.to_string_lossy().to_string());
                    }
                }
            }
            #[cfg(not(unix))]
            {
                return Some(candidate.to_string_lossy().to_string());
            }
        }
    }
    None
}

fn output(program: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(program).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).to_string())
}

fn first_line(text: &str) -> String {
    text.lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .trim()
        .to_string()
}

impl Tools {
    pub fn detect() -> Self {
        let mut t = Tools::default();

        // ImageMagick 7 uses `magick`; older installs expose `convert`.
        let magick_cmd = which("magick").or_else(|| which("convert"));
        if let Some(path) = magick_cmd {
            if let Some(list) = output(&path, &["-list", "format"]) {
                for line in list.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() < 3 {
                        continue;
                    }
                    let perms = parts[2];
                    if !(perms.starts_with('r') || perms.starts_with('-') || perms.starts_with('w'))
                    {
                        continue;
                    }
                    if perms.len() < 2 {
                        continue;
                    }
                    let token = parts[0].trim_end_matches('*').to_uppercase();
                    let readable = perms.starts_with('r');
                    let writable = perms.chars().nth(1) == Some('w');
                    let desc = parts[3..].join(" ");
                    t.magick_formats.insert(token, (readable, writable, desc));
                }
            }
            t.magick_version = first_line(&output(&path, &["-version"]).unwrap_or_default());
            t.magick = Some(path);
        }

        t.ffmpeg = which("ffmpeg");
        if let Some(ref ff) = t.ffmpeg {
            t.ffmpeg_version = first_line(&output(ff, &["-version"]).unwrap_or_default());
            if let Some(enc) = output(ff, &["-hide_banner", "-encoders"]) {
                for line in enc.lines().skip(4) {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 && (parts[0].starts_with('V') || parts[0].starts_with('A') || parts[0].starts_with('S')) {
                        t.encoders.insert(parts[1].to_string());
                    }
                }
            }
            if let Some(mux) = output(ff, &["-hide_banner", "-muxers"]) {
                for line in mux.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 && parts[0].chars().all(|c| c == 'E' || c == 'D') {
                        t.muxers.insert(parts[1].to_string());
                    }
                }
            }
            if let Some(dem) = output(ff, &["-hide_banner", "-demuxers"]) {
                for line in dem.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 && parts[0].chars().all(|c| c == 'D' || c == 'E') {
                        t.demuxers.insert(parts[1].to_string());
                    }
                }
            }
        }
        t.ffprobe = which("ffprobe");

        t.soffice = which("soffice").or_else(|| which("libreoffice"));
        if let Some(ref lo) = t.soffice {
            t.soffice_version = first_line(&output(lo, &["--version"]).unwrap_or_default());
        }

        t.gs = which("gs");
        if let Some(ref gs) = t.gs {
            t.gs_version = first_line(&output(gs, &["--version"]).unwrap_or_default());
        }
        t.pdfinfo = which("pdfinfo");
        t.pdftoppm = which("pdftoppm");
        t.pdftotext = which("pdftotext");
        t.pdftocairo = which("pdftocairo");

        t.sevenzip = which("7z").or_else(|| which("7zz")).or_else(|| which("7za"));
        if let Some(ref seven) = t.sevenzip {
            let info = output(seven, &["i"]).unwrap_or_default();
            t.sevenzip_version = first_line(&info);
            for line in info.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 && parts[0].chars().all(|c| c.is_ascii_digit() || c == '.') {
                    for part in parts.iter().take(4).skip(1) {
                        for token in part.split(',') {
                            let token = token.trim().to_lowercase();
                            if !token.is_empty() && token.len() < 24 {
                                t.sevenzip_formats.insert(token);
                            }
                        }
                    }
                }
            }
        }

        t.tar = which("tar");
        t.zstd = which("zstd");
        t.gzip = which("gzip");
        t.bzip2 = which("bzip2");
        t.xz = which("xz");
        t.file = which("file");
        t.wl_paste = which("wl-paste");
        t.xclip = which("xclip");
        t
    }

    pub fn magick_has(&self, token: &str) -> (bool, bool) {
        match self.magick_formats.get(&token.to_uppercase()) {
            Some((r, w, _)) => (*r, *w),
            None => (false, false),
        }
    }

    pub fn encoder(&self, name: &str) -> bool {
        self.encoders.contains(name)
    }

    pub fn muxer(&self, name: &str) -> bool {
        self.muxers.contains(name)
    }

    pub fn first_encoder(&self, candidates: &[&'static str]) -> Option<&'static str> {
        candidates.iter().copied().find(|c| self.encoders.contains(*c))
    }

    pub fn has_video_encoder(&self, codec: &str) -> bool {
        let candidates: &[&'static str] = match codec {
            "h264" => &["libx264", "libopenh264", "h264_nvenc", "h264_qsv", "h264_vaapi"],
            "hevc" => &["libx265", "hevc_nvenc", "hevc_qsv", "hevc_vaapi"],
            "vp9" => &["libvpx-vp9", "vp9_qsv", "vp9_vaapi"],
            "vp8" => &["libvpx", "vp8_vaapi"],
            "av1" => &["libsvtav1", "libaom-av1", "librav1e", "av1_nvenc", "av1_qsv"],
            "mpeg4" => &["mpeg4", "libxvid"],
            "mpeg2video" => &["mpeg2video"],
            "theora" => &["libtheora"],
            "wmv2" => &["wmv2"],
            "h263" => &["h263", "h263p"],
            "gif" => &["gif"],
            "mjpeg" => &["mjpeg"],
            "png" => &["png"],
            "dvvideo" => &["dvvideo"],
            "prores" => &["prores_ks", "prores_aw", "prores"],
            _ => &[],
        };
        self.first_encoder(candidates).is_some()
    }

    pub fn has_audio_encoder(&self, codec: &str) -> bool {
        let candidates: &[&'static str] = match codec {
            "aac" => &["aac", "libfdk_aac"],
            "mp3" => &["libmp3lame", "libshine", "mp3"],
            "opus" => &["libopus", "opus"],
            "vorbis" => &["libvorbis", "vorbis"],
            "flac" => &["flac"],
            "pcm_s16le" => &["pcm_s16le"],
            "pcm_s16be" => &["pcm_s16be"],
            "ac3" => &["ac3"],
            "eac3" => &["eac3"],
            "wmav2" => &["wmav2"],
            "mp2" => &["mp2", "libtwolame"],
            "amr_nb" => &["libopencore_amrnb"],
            "alac" => &["alac"],
            "wavpack" => &["wavpack"],
            "tta" => &["tta"],
            _ => &[],
        };
        self.first_encoder(candidates).is_some()
    }

    pub fn has_video(&self) -> bool {
        self.ffmpeg.is_some()
    }

    pub fn has_images(&self) -> bool {
        self.magick.is_some()
    }

    pub fn has_documents(&self) -> bool {
        self.soffice.is_some()
    }

    pub fn has_pdf_read(&self) -> bool {
        self.pdftoppm.is_some() && self.pdfinfo.is_some() || self.gs.is_some()
    }

    pub fn has_pdf_text(&self) -> bool {
        self.pdftotext.is_some()
    }

    pub fn has_archives(&self) -> bool {
        self.sevenzip.is_some() || self.tar.is_some()
    }

    pub fn as_tool_infos(&self) -> Vec<ToolInfo> {
        let mut tools = Vec::new();
        let mut push = |id: &str, name: &str, role: &str, path: &Option<String>, version: &str, note: &str| {
            tools.push(ToolInfo {
                id: id.into(),
                name: name.into(),
                role: role.into(),
                available: path.is_some(),
                path: path.clone(),
                version: if path.is_some() && !version.is_empty() { Some(version.to_string()) } else { None },
                note: if note.is_empty() { None } else { Some(note.into()) },
            });
        };
        push("magick", "ImageMagick", "Images, vectors, PDF", &self.magick, &self.magick_version, "");
        push("ffmpeg", "FFmpeg", "Video, audio, subtitles", &self.ffmpeg, &self.ffmpeg_version, "");
        push("ffprobe", "FFprobe", "Media inspection", &self.ffprobe, "", "");
        push("soffice", "LibreOffice", "Documents & office", &self.soffice, &self.soffice_version, "");
        push("gs", "Ghostscript", "PostScript & PDF", &self.gs, &self.gs_version, "");
        push("pdftoppm", "Poppler", "PDF rasterisation", &self.pdftoppm, "", "");
        push("pdftotext", "pdftotext", "PDF text extraction", &self.pdftotext, "", "");
        push("pdftocairo", "pdftocairo", "PDF vector export", &self.pdftocairo, "", "");
        push("7z", "7-Zip", "Archives", &self.sevenzip, &self.sevenzip_version, "");
        push("tar", "tar", "Archive packaging", &self.tar, "", "");
        push("zstd", "Zstandard", "Zstandard archives", &self.zstd, "", "");
        push("gzip", "gzip", "Gzip compression", &self.gzip, "", "");
        push("bzip2", "bzip2", "Bzip2 compression", &self.bzip2, "", "");
        push("xz", "xz", "XZ compression", &self.xz, "", "");
        push("file", "file", "Content sniffing", &self.file, "", "");
        tools
    }
}

pub fn file_exists(path: &Path) -> bool {
    path.is_file()
}

pub fn canonical_or_original(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[derive(Serialize)]
struct Nothing;

#[allow(dead_code)]
fn _keep_serde(_: Nothing) {}
