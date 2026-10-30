pub mod archive;
pub mod document;
pub mod image;
pub mod media;
pub mod run;

use std::path::Path;

use crate::formats::{self, Family, Spec};
use crate::types::Options;

use self::run::TaskCtx;

pub fn execute(
    ctx: &TaskCtx,
    inputs: &[&Path],
    output: &Path,
    target_id: &str,
    source_family: Family,
    options: &Options,
) -> Result<(), String> {
    if inputs.is_empty() {
        return Err("No input files".into());
    }
    let target = formats::lookup(target_id).ok_or_else(|| format!("Unknown target format: {}", target_id))?;
    if target.read_only {
        return Err(format!("{} is not available as an output format", target.label));
    }

    match target.engine {
        formats::Engine::Image => match source_family {
            Family::Video => {
                if target.id == "gif" {
                    media::video_to_gif(ctx, inputs[0], output, target, options)
                } else {
                    media::extract_frame(ctx, inputs[0], output, target, options)
                }
            }
            Family::Audio => media::audio_to_image(ctx, inputs[0], output, target, options),
            Family::Pdf => document::pdf_to_image(ctx, inputs[0], output, target, options),
            Family::Document | Family::Spreadsheet | Family::Presentation => {
                document::convert(ctx, inputs[0], output, target, source_family, options)
            }
            _ => image::convert(ctx, inputs[0], output, target, options),
        },
        formats::Engine::Pdf => match source_family {
            Family::Image | Family::Vector => image::images_to_pdf(ctx, inputs, output, options),
            Family::Document | Family::Spreadsheet | Family::Presentation => {
                document::convert(ctx, inputs[0], output, target, source_family, options)
            }
            _ => Err("This file cannot be composed into a PDF".into()),
        },
        formats::Engine::Media => match source_family {
            Family::Image => {
                if inputs.len() > 1 {
                    media::slideshow(ctx, inputs, output, target, options)
                } else {
                    media::still_to_video(ctx, inputs[0], output, target, options)
                }
            }
            Family::Audio => {
                if target.family == Family::Video {
                    media::audio_to_video(ctx, inputs[0], output, target, options)
                } else {
                    media::convert_audio(ctx, inputs[0], output, target, options)
                }
            }
            Family::Video => media::convert_media(ctx, inputs[0], output, target, options),
            _ => Err("This file cannot be converted to a media format".into()),
        },
        formats::Engine::Document => match source_family {
            Family::Subtitle => media::subtitle_to_text(ctx, inputs[0], output),
            Family::Pdf => document::pdf_to_text(ctx, inputs[0], output),
            _ => document::convert(ctx, inputs[0], output, target, source_family, options),
        },
        formats::Engine::Archive => archive::convert(ctx, inputs[0], output, target, options),
        formats::Engine::Subtitle => media::convert_subtitle(ctx, inputs[0], output, target),
    }
}

/// A short suffix that makes cross-format outputs self-describing.
pub fn output_suffix(source: Family, target: &Spec, merged: bool) -> Option<&'static str> {
    if merged && target.family == Family::Pdf {
        return Some("combined");
    }
    match (source, target.family) {
        (Family::Video, Family::Image) if target.id == "gif" => None,
        (Family::Video, Family::Image) => Some("frame"),
        (Family::Video, Family::Audio) => Some("audio"),
        (Family::Audio, Family::Video) | (Family::Audio, Family::Image) => Some("wave"),
        _ => None,
    }
}
