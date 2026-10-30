use std::path::Path;

use crate::formats::Spec;
use crate::types::Options;

use super::run::{path_arg, run, TaskCtx};

fn magick<'a>(ctx: &'a TaskCtx) -> Result<&'a str, String> {
    ctx.tools
        .magick
        .as_deref()
        .ok_or_else(|| "ImageMagick is not available".to_string())
}

fn is_animated_source(spec: &Spec) -> bool {
    matches!(spec.id, "gif" | "webp" | "apng" | "tiff" | "mng")
}

fn target_keeps_animation(target: &Spec) -> bool {
    matches!(target.id, "gif" | "webp" | "apng")
}

pub fn convert(
    ctx: &TaskCtx,
    input: &Path,
    output: &Path,
    target: &Spec,
    options: &Options,
) -> Result<(), String> {
    ctx.check()?;
    let program = magick(ctx)?;
    let source_spec = crate::formats::from_path(input);
    let preserve_frames = source_spec
        .map(|s| is_animated_source(s) && target_keeps_animation(target))
        .unwrap_or(false);

    let input_arg = if preserve_frames {
        path_arg(input)
    } else {
        format!("{}[0]", path_arg(input))
    };

    let mut args: Vec<String> = Vec::new();
    let is_vector = source_spec
        .map(|s| matches!(s.family, crate::formats::Family::Vector | crate::formats::Family::Pdf))
        .unwrap_or(false);
    if is_vector {
        args.push("-density".into());
        args.push(options.dpi.unwrap_or(144).to_string());
    }

    args.push(input_arg);
    args.push("-auto-orient".into());

    if let Some(max) = options.max_dimension.filter(|m| *m > 0) {
        args.push("-resize".into());
        args.push(format!("{}x{}>", max, max));
    }

    // Flatten transparency for formats that cannot carry an alpha channel.
    if matches!(target.id, "jpg" | "bmp" | "pnm" | "ppm" | "pgm" | "pbm") {
        args.push("-background".into());
        args.push("#ffffff".into());
        args.push("-alpha".into());
        args.push("remove".into());
        args.push("-alpha".into());
        args.push("off".into());
    }

    match target.id {
        "ico" => {
            args.push("-define".into());
            args.push("icon:auto-resize=256,128,64,48,32,16".into());
        }
        "gif" => {
            if preserve_frames {
                args.push("-layers".into());
                args.push("Optimize".into());
            }
            args.push("-loop".into());
            args.push("0".into());
        }
        "png" => {
            args.push("-define".into());
            args.push("png:compression-level=9".into());
        }
        "tiff" => {
            args.push("-compress".into());
            args.push("Zip".into());
        }
        "webp" => {
            args.push("-define".into());
            args.push("webp:method=6".into());
        }
        "apng" => {
            args.push("-define".into());
            args.push("apng:iterations=0".into());
        }
        "pdf" => {
            args.push("-compress".into());
            args.push("jpeg".into());
        }
        _ => {}
    }

    if target.id != "svg" && target.id != "svgz" {
        if options.strip_metadata.unwrap_or(false) {
            args.push("-strip".into());
        } else {
            args.push("-define".into());
            args.push("png:preserve-iCCP=true".into());
        }
    }

    if !target.lossless || matches!(target.id, "webp" | "jpg" | "jxl" | "jp2" | "j2k") {
        args.push("-quality".into());
        args.push(options.quality_or(88).to_string());
    }

    args.push(path_arg(output));

    ctx.report(12.0, "Rendering image");
    run(ctx, program, &args, None)?;

    if !output.exists() {
        return Err("ImageMagick did not produce an output file".into());
    }
    ctx.report(92.0, "Finalising");
    Ok(())
}

pub fn images_to_pdf(
    ctx: &TaskCtx,
    inputs: &[&Path],
    output: &Path,
    options: &Options,
) -> Result<(), String> {
    ctx.check()?;
    if inputs.is_empty() {
        return Err("No images to compose".into());
    }
    let program = magick(ctx)?;
    let mut args: Vec<String> = Vec::new();

    for input in inputs {
        if !input.is_file() {
            return Err(format!("Missing input: {}", input.display()));
        }
        let spec = crate::formats::from_path(input);
        let vector = spec
            .map(|s| matches!(s.family, crate::formats::Family::Vector))
            .unwrap_or(false);
        if vector {
            args.push("-density".into());
            args.push(options.dpi.unwrap_or(150).to_string());
        }
        args.push(path_arg(input));
    }

    args.push("-auto-orient".into());
    args.push("-background".into());
    args.push("#ffffff".into());
    args.push("-alpha".into());
    args.push("remove".into());
    args.push("-alpha".into());
    args.push("off".into());
    args.push("-quality".into());
    args.push(options.quality_or(90).to_string());
    args.push("-compress".into());
    args.push("jpeg".into());
    args.push(path_arg(output));

    ctx.report(18.0, if inputs.len() > 1 { "Composing pages" } else { "Composing PDF" });
    run(ctx, program, &args, None)?;

    if !output.exists() {
        return Err("ImageMagick did not produce a PDF".into());
    }
    ctx.report(92.0, "Finalising");
    Ok(())
}
