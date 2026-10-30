use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

use crate::formats::{self, Family, Spec};
use crate::types::Options;

use super::image;
use super::run::{path_arg, run, run_streaming, TaskCtx};

fn ffmpeg<'a>(ctx: &'a TaskCtx) -> Result<&'a str, String> {
    ctx.tools
        .ffmpeg
        .as_deref()
        .ok_or_else(|| "FFmpeg is not available".to_string())
}

fn ffprobe<'a>(ctx: &'a TaskCtx) -> Result<&'a str, String> {
    ctx.tools
        .ffprobe
        .as_deref()
        .ok_or_else(|| "FFprobe is not available".to_string())
}

#[derive(Default, Debug, Clone)]
struct MediaInfo {
    duration: Option<f64>,
    has_audio: bool,
    has_video: bool,
    width: Option<u32>,
    height: Option<u32>,
    bitrate: Option<u64>,
}

fn probe(ctx: &TaskCtx, path: &Path) -> MediaInfo {
    let mut info = MediaInfo::default();
    let Ok(probe) = ffprobe(ctx) else {
        return info;
    };
    let output = Command::new(probe)
        .args([
            "-v",
            "error",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
            &path_arg(path),
        ])
        .output();
    let Ok(output) = output else { return info };
    if !output.status.success() {
        return info;
    }
    let Ok(json) = serde_json::from_slice::<Value>(&output.stdout) else {
        return info;
    };
    if let Some(streams) = json["streams"].as_array() {
        for stream in streams {
            match stream["codec_type"].as_str().unwrap_or("") {
                "video" => {
                    let attached = stream["disposition"]["attached_pic"].as_u64().unwrap_or(0);
                    if attached == 0 {
                        info.has_video = true;
                        info.width = stream["width"].as_u64().map(|v| v as u32);
                        info.height = stream["height"].as_u64().map(|v| v as u32);
                    }
                }
                "audio" => info.has_audio = true,
                _ => {}
            }
        }
    }
    info.duration = json["format"]["duration"]
        .as_str()
        .and_then(|v| v.parse::<f64>().ok())
        .or_else(|| {
            json["streams"]
                .as_array()
                .and_then(|s| s.iter().find_map(|x| x["duration"].as_str().and_then(|v| v.parse::<f64>().ok())))
        });
    info.bitrate = json["format"]["bit_rate"]
        .as_str()
        .and_then(|v| v.parse::<u64>().ok());
    info
}

fn even(value: u32) -> u32 {
    value & !1
}

fn trim_span(info: &MediaInfo, options: &Options) -> Option<f64> {
    let start = options.trim_start.unwrap_or(0.0).max(0.0);
    let end = options.trim_end.filter(|v| *v > start);
    let total = end.map(|e| e - start).or_else(|| info.duration.map(|d| (d - start).max(0.0)));
    total.filter(|t| *t > 0.0)
}

fn input_args(input: &Path, options: &Options, duration: Option<f64>) -> Vec<String> {
    let mut args = vec![
        "-hide_banner".to_string(),
        "-loglevel".into(),
        "error".into(),
        "-nostdin".into(),
        "-y".into(),
        "-progress".into(),
        "pipe:1".into(),
    ];
    let start = options.trim_start.unwrap_or(0.0);
    if start > 0.0 {
        args.push("-ss".into());
        args.push(format!("{:.3}", start));
    }
    args.push("-i".into());
    args.push(path_arg(input));
    if let Some(duration) = duration {
        if options.trim_start.is_some() || options.trim_end.is_some() {
            args.push("-t".into());
            args.push(format!("{:.3}", duration));
        }
    }
    args
}

fn quality_crf(quality: u8) -> u8 {
    let q = (quality.clamp(1, 100) as f32) / 100.0;
    (34.0 - q * 18.0).round().clamp(14.0, 34.0) as u8
}

fn quality_vpx(quality: u8) -> u8 {
    let q = (quality.clamp(1, 100) as f32) / 100.0;
    (60.0 - q * 42.0).round().clamp(15.0, 60.0) as u8
}

fn bitrate_for(height: u32, quality: u8) -> u64 {
    let base = match height {
        h if h >= 2000 => 18_000_000,
        h if h >= 1400 => 10_000_000,
        h if h >= 1000 => 5_000_000,
        h if h >= 700 => 2_500_000,
        _ => 1_200_000,
    };
    let scale = 0.6 + (quality.clamp(1, 100) as f64 / 100.0) * 0.9;
    (base as f64 * scale) as u64
}

fn pick_video_encoder(ctx: &TaskCtx, codec: &str) -> Result<&'static str, String> {
    let candidates: &[&'static str] = match codec {
        "h264" => &["libx264", "libopenh264"],
        "hevc" => &["libx265"],
        "vp9" => &["libvpx-vp9"],
        "vp8" => &["libvpx"],
        "av1" => &["libsvtav1", "libaom-av1", "librav1e"],
        "mpeg4" => &["mpeg4", "libxvid"],
        "mpeg2video" => &["mpeg2video"],
        "theora" => &["libtheora"],
        "wmv2" => &["wmv2"],
        "h263" => &["h263"],
        "dvvideo" => &["dvvideo"],
        "prores" => &["prores_ks", "prores"],
        _ => &[],
    };
    ctx.tools
        .first_encoder(candidates)
        .ok_or_else(|| format!("No encoder available for codec {}", codec))
}

fn video_codec_args(
    ctx: &TaskCtx,
    codec: &str,
    quality: u8,
    height: u32,
    fps: u32,
) -> Result<Vec<String>, String> {
    let encoder = pick_video_encoder(ctx, codec)?;
    let mut args = vec!["-c:v".to_string(), encoder.to_string()];
    let crf = quality_crf(quality);
    let bitrate = bitrate_for(height, quality);

    match encoder {
        "libx264" | "libx265" => {
            args.push("-preset".into());
            args.push("medium".into());
            args.push("-crf".into());
            args.push(crf.to_string());
            args.push("-pix_fmt".into());
            args.push("yuv420p".into());
        }
        "libopenh264" => {
            args.push("-b:v".into());
            args.push(bitrate.to_string());
            args.push("-pix_fmt".into());
            args.push("yuv420p".into());
            args.push("-g".into());
            args.push((fps * 2).to_string());
        }
        "libvpx-vp9" => {
            args.push("-crf".into());
            args.push(quality_vpx(quality).to_string());
            args.push("-b:v".into());
            args.push("0".into());
            args.push("-row-mt".into());
            args.push("1".into());
            args.push("-cpu-used".into());
            args.push("2".into());
            args.push("-deadline".into());
            args.push("good".into());
            args.push("-pix_fmt".into());
            args.push("yuv420p".into());
        }
        "libvpx" => {
            args.push("-crf".into());
            args.push(quality_vpx(quality).to_string());
            args.push("-b:v".into());
            args.push(bitrate.to_string());
            args.push("-pix_fmt".into());
            args.push("yuv420p".into());
        }
        "libsvtav1" => {
            args.push("-crf".into());
            args.push(crf.to_string());
            args.push("-preset".into());
            args.push("8".into());
            args.push("-pix_fmt".into());
            args.push("yuv420p".into());
        }
        "libaom-av1" => {
            args.push("-crf".into());
            args.push(crf.to_string());
            args.push("-b:v".into());
            args.push("0".into());
            args.push("-cpu-used".into());
            args.push("6".into());
            args.push("-pix_fmt".into());
            args.push("yuv420p".into());
        }
        "mpeg4" | "libxvid" | "mpeg2video" => {
            args.push("-q:v".into());
            args.push((2 + (100u32.saturating_sub(quality as u32)) / 6).to_string());
        }
        "wmv2" => {
            args.push("-b:v".into());
            args.push(bitrate.to_string());
        }
        "libtheora" => {
            args.push("-q:v".into());
            args.push((1 + (quality as u32 * 9 / 100)).to_string());
        }
        "h263" => {
            args.push("-b:v".into());
            args.push(bitrate.to_string());
        }
        "dvvideo" => {
            args.push("-s".into());
            args.push("720x576".into());
            args.push("-r".into());
            args.push("25".into());
            args.push("-pix_fmt".into());
            args.push("yuv420p".into());
        }
        _ => {
            args.push("-b:v".into());
            args.push(bitrate.to_string());
            args.push("-pix_fmt".into());
            args.push("yuv420p".into());
        }
    }
    Ok(args)
}

fn pick_audio_encoder(ctx: &TaskCtx, codec: &str) -> Result<&'static str, String> {
    let candidates: &[&'static str] = match codec {
        "aac" => &["aac", "libfdk_aac"],
        "mp3" => &["libmp3lame", "mp3"],
        "opus" => &["libopus"],
        "vorbis" => &["libvorbis"],
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
    ctx.tools
        .first_encoder(candidates)
        .ok_or_else(|| format!("No audio encoder available for {}", codec))
}

/// MPEG-1 Layer II only accepts a fixed set of bitrates; FFmpeg refuses
/// anything else, so snap the slider value to the nearest legal step.
fn mpeg_audio_bitrate(bitrate: u32) -> u32 {
    const ALLOWED: [u32; 14] = [32, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384];
    ALLOWED
        .into_iter()
        .min_by_key(|step| step.abs_diff(bitrate))
        .unwrap_or(224)
}

fn audio_codec_args(
    ctx: &TaskCtx,
    codec: &str,
    options: &Options,
    quality: u8,
) -> Result<Vec<String>, String> {
    let encoder = pick_audio_encoder(ctx, codec)?;
    let mut args = vec!["-c:a".to_string(), encoder.to_string()];
    let default_bitrate = (96 + quality as u32 * 3 / 2).clamp(96, 320);
    let bitrate = options.audio_bitrate.unwrap_or(default_bitrate);

    match encoder {
        "libmp3lame" | "libfdk_aac" | "libopus" | "libvorbis" | "wmav2" => {
            args.push("-b:a".into());
            args.push(format!("{}k", bitrate));
            if encoder == "libopus" {
                args.push("-vbr".into());
                args.push("on".into());
            }
        }
        "mp2" | "libtwolame" => {
            args.push("-b:a".into());
            args.push(format!("{}k", mpeg_audio_bitrate(bitrate)));
        }
        "aac" | "ac3" | "eac3" => {
            args.push("-b:a".into());
            args.push(format!("{}k", bitrate.max(128)));
        }
        "flac" => {
            args.push("-compression_level".into());
            args.push((12u32.saturating_sub(quality as u32 / 10)).max(4).to_string());
        }
        "amr_nb" => {
            args.push("-ar".into());
            args.push("8000".into());
            args.push("-ac".into());
            args.push("1".into());
            args.push("-b:a".into());
            args.push("12.2k".into());
        }
        "wavpack" | "tta" => {
            args.push("-compression_level".into());
            args.push("4".into());
        }
        _ => {}
    }

    if codec != "amr_nb" {
        if let Some(rate) = options.sample_rate.filter(|r| *r > 0) {
            args.push("-ar".into());
            args.push(rate.to_string());
        }
        if let Some(channels) = options.channels.filter(|c| (1..=8).contains(c)) {
            args.push("-ac".into());
            args.push(channels.to_string());
        }
    }
    if options.normalize_audio.unwrap_or(false) {
        args.push("-af".into());
        args.push("loudnorm=I=-16:TP=-1.5:LRA=11".into());
    }
    Ok(args)
}

fn run_ffmpeg(
    ctx: &TaskCtx,
    args: &[String],
    total: Option<f64>,
    stage: &str,
) -> Result<(), String> {
    let program = ffmpeg(ctx)?;
    let mut last = 0f32;
    let stage = stage.to_string();
    run_streaming(ctx, program, args, None, |line| {
        let microseconds = line
            .strip_prefix("out_time_us=")
            .or_else(|| line.strip_prefix("out_time_ms="))
            .and_then(|v| v.trim().parse::<f64>().ok());
        if let Some(micros) = microseconds {
            if let Some(total) = total.filter(|t| *t > 0.0) {
                let seconds = micros / 1_000_000.0;
                let pct = 8.0 + (seconds / total).clamp(0.0, 1.0) as f32 * 86.0;
                if pct > last + 0.4 {
                    last = pct;
                    ctx.report(pct, &stage);
                }
            }
        } else if line.starts_with("progress=end") {
            ctx.report(96.0, &stage);
        }
    })?;
    Ok(())
}

fn muxer_args(target: &Spec) -> Vec<String> {
    let mut args = Vec::new();
    if let Some(muxer) = target.ff_muxer {
        args.push("-f".into());
        args.push(muxer.into());
    }
    if matches!(target.id, "mp4" | "mov" | "m4v" | "m4a" | "m4b") {
        args.push("-movflags".into());
        args.push("+faststart".into());
    }
    args
}

fn output_ready(output: &Path) -> Result<(), String> {
    if output.exists() && output.metadata().map(|m| m.len() > 0).unwrap_or(false) {
        Ok(())
    } else {
        Err("The encoder did not produce an output file".into())
    }
}

pub fn convert_media(
    ctx: &TaskCtx,
    input: &Path,
    output: &Path,
    target: &Spec,
    options: &Options,
) -> Result<(), String> {
    ctx.check()?;
    let info = probe(ctx, input);
    if target.family == Family::Audio {
        return encode_audio(ctx, input, output, target, options, &info);
    }
    if !info.has_video {
        return Err("This file has no video stream".into());
    }

    let total = trim_span(&info, options);
    let has_audio = info.has_audio
        && !options.mute.unwrap_or(false)
        && target.ff_audio.is_some();
    let quality = options.quality_or(85);
    let height = options
        .video_height
        .filter(|h| *h > 0)
        .unwrap_or(info.height.unwrap_or(1080));
    let fps = options.fps.filter(|f| *f > 0).unwrap_or(30);
    let mut args = input_args(input, options, total);

    let mut filters: Vec<String> = Vec::new();
    if options.video_height.filter(|h| *h > 0).is_some() {
        filters.push(format!("scale=-2:{}:flags=lanczos", even(height).max(2)));
    }
    if options.fps.filter(|f| *f > 0).is_some() {
        filters.push(format!("fps={}", fps));
    }
    if !filters.is_empty() {
        args.push("-vf".into());
        args.push(filters.join(","));
    }

    args.push("-map".into());
    args.push("0:v:0".into());
    if has_audio {
        args.push("-map".into());
        args.push("0:a:0".into());
    }
    args.extend(video_codec_args(ctx, target.ff_video.unwrap_or("h264"), quality, height, fps)?);
    if has_audio {
        args.extend(audio_codec_args(
            ctx,
            target.ff_audio.unwrap_or("aac"),
            options,
            quality,
        )?);
        // The MXF muxer only accepts 48 kHz PCM.
        if target.ff_muxer == Some("mxf") {
            args.push("-ar".into());
            args.push("48000".into());
        }
    } else {
        args.push("-an".into());
    }
    args.extend(muxer_args(target));
    args.push(path_arg(output));

    ctx.report(4.0, "Preparing");
    run_ffmpeg(ctx, &args, total, "Encoding")?;
    output_ready(output)?;
    ctx.report(97.0, "Finalising");
    Ok(())
}

fn encode_audio(
    ctx: &TaskCtx,
    input: &Path,
    output: &Path,
    target: &Spec,
    options: &Options,
    info: &MediaInfo,
) -> Result<(), String> {
    if !info.has_audio {
        return Err("This file has no audio stream".into());
    }
    let total = trim_span(info, options);
    let quality = options.quality_or(85);
    let mut args = input_args(input, options, total);
    args.push("-vn".into());
    args.push("-map".into());
    args.push("0:a:0".into());
    args.extend(audio_codec_args(ctx, target.ff_audio.unwrap_or("mp3"), options, quality)?);
    args.extend(muxer_args(target));
    args.push(path_arg(output));
    ctx.report(4.0, "Preparing audio");
    run_ffmpeg(ctx, &args, total, "Encoding audio")?;
    output_ready(output)?;
    ctx.report(97.0, "Finalising");
    Ok(())
}

pub fn convert_audio(
    ctx: &TaskCtx,
    input: &Path,
    output: &Path,
    target: &Spec,
    options: &Options,
) -> Result<(), String> {
    let info = probe(ctx, input);
    encode_audio(ctx, input, output, target, options, &info)
}

pub fn video_to_gif(
    ctx: &TaskCtx,
    input: &Path,
    output: &Path,
    _target: &Spec,
    options: &Options,
) -> Result<(), String> {
    let info = probe(ctx, input);
    if !info.has_video {
        return Err("This file has no video stream".into());
    }
    let total = trim_span(&info, options);
    let fps = options.fps.unwrap_or(12).clamp(4, 30);
    // Stay near 480p for weight, but never upscale a smaller source.
    let source_height = info.height.unwrap_or(480);
    let wanted = options.video_height.unwrap_or(480).clamp(120, 1080);
    let height = even(wanted.min(source_height)).max(2);
    let filter = format!(
        "fps={},scale=-1:{}:flags=lanczos,split[s0][s1];[s0]palettegen=max_colors=256:stats_mode=diff[p];[s1][p]paletteuse=dither=sierra2_4a",
        fps, height
    );
    let mut args = input_args(input, options, total);
    args.push("-vf".into());
    args.push(filter);
    args.push("-an".into());
    args.push("-loop".into());
    args.push("0".into());
    args.push("-f".into());
    args.push("gif".into());
    args.push(path_arg(output));
    ctx.report(4.0, "Building palette");
    run_ffmpeg(ctx, &args, total, "Building GIF")?;
    output_ready(output)?;
    ctx.report(97.0, "Finalising");
    Ok(())
}

pub fn extract_frame(
    ctx: &TaskCtx,
    input: &Path,
    output: &Path,
    target: &Spec,
    options: &Options,
) -> Result<(), String> {
    let info = probe(ctx, input);
    if !info.has_video {
        return Err("This file has no video stream".into());
    }
    let duration = info.duration.unwrap_or(0.0);
    let mut time = options.frame_time.unwrap_or(if duration > 2.0 { 1.0 } else { 0.0 });
    if duration > 0.05 {
        time = time.clamp(0.0, (duration - 0.05).max(0.0));
    }
    let temp = ctx.temp_file(&format!("frame-{}.png", uuid::Uuid::new_v4()));
    let args = vec![
        "-hide_banner".to_string(),
        "-loglevel".into(),
        "error".into(),
        "-nostdin".into(),
        "-y".into(),
        "-ss".into(),
        format!("{:.3}", time),
        "-i".into(),
        path_arg(input),
        "-frames:v".into(),
        "1".into(),
        "-vsync".into(),
        "0".into(),
        path_arg(&temp),
    ];
    ctx.report(20.0, "Seeking frame");
    run(ctx, ffmpeg(ctx)?, &args, None)?;
    if !temp.exists() {
        return Err("Could not extract a frame at that position".into());
    }
    ctx.report(55.0, "Rendering frame");
    image::convert(ctx, &temp, output, target, options)?;
    let _ = std::fs::remove_file(&temp);
    Ok(())
}

pub fn audio_to_image(
    ctx: &TaskCtx,
    input: &Path,
    output: &Path,
    target: &Spec,
    options: &Options,
) -> Result<(), String> {
    let info = probe(ctx, input);
    if !info.has_audio {
        return Err("This file has no audio stream".into());
    }
    let temp = ctx.temp_file(&format!("wave-{}.png", uuid::Uuid::new_v4()));
    let args = vec![
        "-hide_banner".to_string(),
        "-loglevel".into(),
        "error".into(),
        "-nostdin".into(),
        "-y".into(),
        "-i".into(),
        path_arg(input),
        "-filter_complex".into(),
        "aformat=channel_layouts=mono,showwavespic=s=1600x500:colors=0xF0A35E".into(),
        "-frames:v".into(),
        "1".into(),
        path_arg(&temp),
    ];
    ctx.report(20.0, "Drawing waveform");
    run(ctx, ffmpeg(ctx)?, &args, None)?;
    if !temp.exists() {
        return Err("Could not render the waveform".into());
    }
    ctx.report(60.0, "Rendering image");
    image::convert(ctx, &temp, output, target, options)?;
    let _ = std::fs::remove_file(&temp);
    Ok(())
}

fn canvas(height: u32) -> (u32, u32) {
    let h = even(height.clamp(240, 2160)).max(2);
    let w = even(((h as f64) * 16.0 / 9.0).round() as u32).max(2);
    (w, h)
}

pub fn still_to_video(
    ctx: &TaskCtx,
    input: &Path,
    output: &Path,
    target: &Spec,
    options: &Options,
) -> Result<(), String> {
    let duration = options.still_duration.unwrap_or(5.0).clamp(0.5, 600.0);
    let fps = options.fps.unwrap_or(30).clamp(10, 60);
    let height = options.video_height.unwrap_or(1080);
    let (w, h) = canvas(height);
    let quality = options.quality_or(85);
    let filter = format!(
        "scale={}:{}:force_original_aspect_ratio=decrease,pad={}:{}:(ow-iw)/2:(oh-ih)/2:color=0x0b0b0c,format=yuv420p",
        w, h, w, h
    );
    let mut args: Vec<String> = vec![
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-nostdin".into(),
        "-y".into(),
        "-progress".into(),
        "pipe:1".into(),
        "-loop".into(),
        "1".into(),
        "-i".into(),
        path_arg(input),
        "-t".into(),
        format!("{:.3}", duration),
        "-r".into(),
        fps.to_string(),
        "-vf".into(),
        filter,
    ];
    args.extend(video_codec_args(ctx, target.ff_video.unwrap_or("h264"), quality, h, fps)?);
    args.push("-an".into());
    args.extend(muxer_args(target));
    args.push(path_arg(output));
    ctx.report(5.0, "Building video");
    run_ffmpeg(ctx, &args, Some(duration), "Building video")?;
    output_ready(output)?;
    ctx.report(97.0, "Finalising");
    Ok(())
}

fn escape_concat(path: &Path) -> String {
    path.to_string_lossy().replace('\'', "'\\''")
}

pub fn slideshow(
    ctx: &TaskCtx,
    inputs: &[&Path],
    output: &Path,
    target: &Spec,
    options: &Options,
) -> Result<(), String> {
    if inputs.is_empty() {
        return Err("No images for the slideshow".into());
    }
    let per_image = options.still_duration.unwrap_or(4.0).clamp(0.4, 120.0);
    let list_path = ctx.temp_file(&format!("slideshow-{}.txt", uuid::Uuid::new_v4()));
    let mut list = String::new();
    for input in inputs {
        list.push_str(&format!("file '{}'\nduration {:.3}\n", escape_concat(input), per_image));
    }
    // The concat demuxer needs the final file repeated to honour the last duration.
    if let Some(last) = inputs.last() {
        list.push_str(&format!("file '{}'\n", escape_concat(last)));
    }
    std::fs::write(&list_path, list).map_err(|e| format!("Could not stage the slideshow: {}", e))?;

    let fps = options.fps.unwrap_or(30).clamp(10, 60);
    let height = options.video_height.unwrap_or(1080);
    let (w, h) = canvas(height);
    let quality = options.quality_or(85);
    let filter = format!(
        "scale={}:{}:force_original_aspect_ratio=decrease,pad={}:{}:(ow-iw)/2:(oh-ih)/2:color=0x0b0b0c,format=yuv420p",
        w, h, w, h
    );
    let total = per_image * inputs.len() as f64;
    let mut args: Vec<String> = vec![
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-nostdin".into(),
        "-y".into(),
        "-progress".into(),
        "pipe:1".into(),
        "-f".into(),
        "concat".into(),
        "-safe".into(),
        "0".into(),
        "-i".into(),
        path_arg(&list_path),
        "-vf".into(),
        filter,
        "-r".into(),
        fps.to_string(),
    ];
    args.extend(video_codec_args(ctx, target.ff_video.unwrap_or("h264"), quality, h, fps)?);
    args.push("-an".into());
    args.extend(muxer_args(target));
    args.push(path_arg(output));
    ctx.report(5.0, "Building slideshow");
    run_ffmpeg(ctx, &args, Some(total), "Building slideshow")?;
    output_ready(output)?;
    let _ = std::fs::remove_file(&list_path);
    ctx.report(97.0, "Finalising");
    Ok(())
}

pub fn audio_to_video(
    ctx: &TaskCtx,
    input: &Path,
    output: &Path,
    target: &Spec,
    options: &Options,
) -> Result<(), String> {
    let info = probe(ctx, input);
    if !info.has_audio {
        return Err("This file has no audio stream".into());
    }
    let duration = trim_span(&info, options).or(info.duration).ok_or("Could not determine audio duration")?;
    let height = options.video_height.unwrap_or(720);
    let (w, h) = canvas(height);
    let wave_w = even((w as f64 * 0.76) as u32).max(2);
    let wave_h = even((h as f64 * 0.38) as u32).max(2);
    let quality = options.quality_or(85);
    let fps = 30u32;
    let start = options.trim_start.unwrap_or(0.0);

    let mut args: Vec<String> = vec![
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-nostdin".into(),
        "-y".into(),
        "-progress".into(),
        "pipe:1".into(),
        "-f".into(),
        "lavfi".into(),
        "-i".into(),
        format!("color=c=0x0b0b0c:s={}x{}:r={}:d={:.3}", w, h, fps, duration),
    ];
    if start > 0.0 {
        args.push("-ss".into());
        args.push(format!("{:.3}", start));
    }
    args.push("-i".into());
    args.push(path_arg(input));
    let filter = format!(
        "[1:a]showwaves=s={}x{}:mode=cline:rate={}:colors=0xF0A35E,format=yuva420p[w];[0:v][w]overlay=(W-w)/2:(H-h)/2,format=yuv420p[v]",
        wave_w, wave_h, fps
    );
    args.push("-filter_complex".into());
    args.push(filter);
    args.push("-map".into());
    args.push("[v]".into());
    args.push("-map".into());
    args.push("1:a:0".into());
    args.extend(video_codec_args(ctx, target.ff_video.unwrap_or("h264"), quality, h, fps)?);
    args.extend(audio_codec_args(
        ctx,
        target.ff_audio.unwrap_or("aac"),
        options,
        quality,
    )?);
    args.push("-t".into());
    args.push(format!("{:.3}", duration));
    args.push("-shortest".into());
    args.extend(muxer_args(target));
    args.push(path_arg(output));

    ctx.report(4.0, "Drawing waveform");
    run_ffmpeg(ctx, &args, Some(duration), "Building visualiser")?;
    output_ready(output)?;
    ctx.report(97.0, "Finalising");
    Ok(())
}

fn subtitle_codec(target: &Spec) -> &'static str {
    match target.id {
        "vtt" => "webvtt",
        "ass" | "ssa" => "ass",
        "ttml" => "ttml",
        _ => "subrip",
    }
}

pub fn convert_subtitle(
    ctx: &TaskCtx,
    input: &Path,
    output: &Path,
    target: &Spec,
) -> Result<(), String> {
    let args = vec![
        "-hide_banner".to_string(),
        "-loglevel".into(),
        "error".into(),
        "-nostdin".into(),
        "-y".into(),
        "-i".into(),
        path_arg(input),
        "-map".into(),
        "0:s:0".into(),
        "-c:s".into(),
        subtitle_codec(target).into(),
        "-f".into(),
        target.ff_muxer.unwrap_or("srt").into(),
        path_arg(output),
    ];
    ctx.report(25.0, "Reading subtitles");
    run(ctx, ffmpeg(ctx)?, &args, None)?;
    output_ready(output)?;
    ctx.report(95.0, "Finalising");
    Ok(())
}

fn strip_markup(text: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for ch in text.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&#39;", "'")
        .replace("&quot;", "\"")
        .replace("&nbsp;", " ")
}

pub fn subtitle_to_text(ctx: &TaskCtx, input: &Path, output: &Path) -> Result<(), String> {
    let temp = ctx.temp_file(&format!("subtitle-{}.srt", uuid::Uuid::new_v4()));
    let target = formats::lookup("srt").ok_or("Subtitle support is unavailable")?;
    convert_subtitle(ctx, input, &temp, target)?;
    let raw = std::fs::read_to_string(&temp).map_err(|e| format!("Could not read subtitles: {}", e))?;
    let mut text = String::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !text.ends_with("\n\n") && !text.is_empty() {
                text.push('\n');
            }
            continue;
        }
        if trimmed.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        if trimmed.contains("-->") {
            continue;
        }
        text.push_str(&strip_markup(trimmed));
        text.push('\n');
    }
    std::fs::write(output, text.trim().to_string() + "\n")
        .map_err(|e| format!("Could not write text: {}", e))?;
    let _ = std::fs::remove_file(&temp);
    ctx.report(95.0, "Finalising");
    Ok(())
}

#[allow(dead_code)]
fn _unused(_: PathBuf) {}
