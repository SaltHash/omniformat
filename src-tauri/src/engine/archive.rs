use std::path::{Path, PathBuf};

use super::run::{path_arg, run, TaskCtx};
use crate::formats::Spec;
use crate::types::Options;

/// Archive → archive conversion: extract into scratch space, then repack the
/// contents in the requested container. 7-Zip handles almost every input
/// (including RAR, ISO, DEB and RPM); tar is used as a fallback for tar-family
/// files when 7-Zip is unavailable.
/// Archive tools are launched from the scratch directory, so any relative
/// path has to be resolved against the caller's working directory first.
fn absolute(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|dir| dir.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    }
}

pub fn convert(
    ctx: &TaskCtx,
    input: &Path,
    output: &Path,
    target: &Spec,
    options: &Options,
) -> Result<(), String> {
    let input = absolute(input);
    let output = absolute(output);
    let input = input.as_path();
    let output = output.as_path();
    let name = input
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let tar_like = name.ends_with(".tar")
        || name.ends_with(".tar.gz")
        || name.ends_with(".tgz")
        || name.ends_with(".tar.bz2")
        || name.ends_with(".tbz2")
        || name.ends_with(".tar.xz")
        || name.ends_with(".txz")
        || name.ends_with(".tar.zst")
        || name.ends_with(".tzst");

    let extract = ctx.temp.join("archive-extract");
    if extract.exists() {
        let _ = std::fs::remove_dir_all(&extract);
    }
    std::fs::create_dir_all(&extract)
        .map_err(|e| format!("Could not create scratch folder: {}", e))?;

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Could not create output folder: {}", e))?;
    }
    let _ = std::fs::remove_file(output);

    ctx.check()?;
    ctx.report(5.0, "Reading archive");

    let seven = ctx.tools.sevenzip.clone();
    let tar = ctx.tools.tar.clone();

    if let Some(ref seven) = seven {
        ctx.report(18.0, "Extracting");
        let args = vec![
            "x".to_string(),
            "-y".to_string(),
            format!("-o{}", path_arg(&extract)),
            path_arg(input),
        ];
        match run(ctx, seven, &args, None) {
            Ok(_) => {}
            Err(err) if tar_like && tar.is_some() => {
                // Some tar variants are better handled by tar itself; retry
                // only after clearing the partial extraction.
                let _ = std::fs::remove_dir_all(&extract);
                std::fs::create_dir_all(&extract)
                    .map_err(|e| format!("Could not reset scratch folder: {}", e))?;
                let tar = tar.as_ref().unwrap();
                let args = vec![
                    "-xf".to_string(),
                    path_arg(input),
                    "-C".to_string(),
                    path_arg(&extract),
                ];
                run(ctx, tar, &args, None)
                    .map_err(|e| format!("Could not extract archive: {}", e))?;
                let _ = err;
            }
            Err(err) => return Err(format!("Could not extract archive: {}", err)),
        }
    } else if let Some(ref tar) = tar {
        if !tar_like {
            return Err("No 7-Zip found; only tar archives can be read".to_string());
        }
        ctx.report(18.0, "Extracting");
        let args = vec![
            "-xf".to_string(),
            path_arg(input),
            "-C".to_string(),
            path_arg(&extract),
        ];
        run(ctx, tar, &args, None)
            .map_err(|e| format!("Could not extract archive: {}", e))?;
    } else {
        return Err("No archive tool available (install 7-Zip or tar)".to_string());
    }

    ctx.check()?;
    ctx.report(48.0, "Packing");

    let level = options.compression.unwrap_or(5).clamp(1, 9);

    match target.id {
        "zip" | "7z" => {
            let seven = ctx
                .tools
                .sevenzip
                .as_ref()
                .ok_or_else(|| "7-Zip is required to create ZIP or 7Z archives".to_string())?;
            let kind = if target.id == "zip" { "zip" } else { "7z" };
            let args = vec![
                "a".to_string(),
                format!("-t{}", kind),
                format!("-mx={}", level),
                "-y".to_string(),
                path_arg(output),
                ".".to_string(),
            ];
            run(ctx, seven, &args, Some(&extract))
                .map_err(|e| format!("Could not create {} archive: {}", target.label, e))?;
        }
        "tar" => {
            let tar = ctx
                .tools
                .tar
                .as_ref()
                .ok_or_else(|| "tar is not available".to_string())?;
            let args = vec![
                "-cf".to_string(),
                path_arg(output),
                "-C".to_string(),
                path_arg(&extract),
                ".".to_string(),
            ];
            run(ctx, tar, &args, None)
                .map_err(|e| format!("Could not create tar archive: {}", e))?;
        }
        "tar.gz" | "tar.bz2" | "tar.xz" | "tar.zst" => {
            let tar = ctx
                .tools
                .tar
                .as_ref()
                .ok_or_else(|| "tar is not available".to_string())?;
            let mut args: Vec<String> = Vec::new();
            match target.id {
                "tar.gz" => args.push("-czf".to_string()),
                "tar.bz2" => args.push("-cjf".to_string()),
                "tar.xz" => args.push("-cJf".to_string()),
                "tar.zst" => {
                    args.push("--zstd".to_string());
                    args.push("-cf".to_string());
                }
                _ => unreachable!(),
            }
            args.extend([
                path_arg(output),
                "-C".to_string(),
                path_arg(&extract),
                ".".to_string(),
            ]);
            run(ctx, tar, &args, None)
                .map_err(|e| format!("Could not create {} archive: {}", target.label, e))?;
        }
        other => {
            return Err(format!("Unsupported archive target: {}", other));
        }
    }

    ctx.check()?;
    ctx.report(95.0, "Finalising");

    let meta = std::fs::metadata(output)
        .map_err(|e| format!("Archive was not created: {}", e))?;
    if !meta.is_file() || meta.len() == 0 {
        return Err("Archive creation produced an empty file".to_string());
    }

    Ok(())
}

