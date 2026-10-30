use std::collections::HashMap;
use std::fs::{self, File};
use std::path::{Component, Path, PathBuf};

use uuid::Uuid;
use zip::ZipArchive;

use crate::engine::image;
use crate::engine::run::{path_arg, run, TaskCtx};
use crate::formats::{Family, Spec};
use crate::types::Options;

/// Convert an office/document file through LibreOffice. EPUB input is first
/// unpacked into a single HTML document because LibreOffice has no EPUB import.
pub fn convert(
    ctx: &TaskCtx,
    input: &Path,
    output: &Path,
    target: &Spec,
    source_family: Family,
    options: &Options,
) -> Result<(), String> {
    ctx.check()?;
    ctx.report(6.0, "Preparing");

    if ctx.tools.soffice.is_none() {
        return Err("LibreOffice is not available".to_string());
    }

    let is_epub = input
        .extension()
        .map(|e| e.to_string_lossy().eq_ignore_ascii_case("epub"))
        .unwrap_or(false);

    let effective_input = if is_epub {
        let html = epub_to_html(ctx, input)?;
        if target.id == "html" {
            ctx.report(90.0, "Writing HTML");
            write_output(&html, output)?;
            ctx.report(98.0, "Done");
            return Ok(());
        }
        html
    } else {
        input.to_path_buf()
    };

    let _ = options;
    libreoffice_convert(ctx, &effective_input, output, target, source_family)
}

fn libreoffice_convert(
    ctx: &TaskCtx,
    input: &Path,
    output: &Path,
    target: &Spec,
    source_family: Family,
) -> Result<(), String> {
    let program = ctx
        .tools
        .soffice
        .clone()
        .ok_or_else(|| "LibreOffice is not available".to_string())?;

    let work = ctx.temp.join(format!("lo-{}", Uuid::new_v4().simple()));
    let outdir = work.join("out");
    fs::create_dir_all(&outdir).map_err(|e| format!("Could not create scratch directory: {}", e))?;

    // A private profile lets concurrent jobs run without fighting over locks.
    let profile = std::env::temp_dir().join(format!("convert-lo-profile-{}", Uuid::new_v4().simple()));
    fs::create_dir_all(&profile).map_err(|e| format!("Could not create LibreOffice profile: {}", e))?;

    let (filter, expected_ext) = lo_filter(target.id, source_family);
    let args = vec![
        "--headless".to_string(),
        "--norestore".to_string(),
        "--invisible".to_string(),
        "--nodefault".to_string(),
        "--nolockcheck".to_string(),
        "--nologo".to_string(),
        format!("-env:UserInstallation={}", file_url(&profile)),
        "--convert-to".to_string(),
        filter,
        "--outdir".to_string(),
        path_arg(&outdir),
        path_arg(input),
    ];

    ctx.report(30.0, "Converting document");
    let result = run(ctx, &program, &args, None);
    let _ = fs::remove_dir_all(&profile);

    let stdout = match result {
        Ok(out) => out,
        Err(err) => {
            let _ = fs::remove_dir_all(&work);
            return Err(format!("LibreOffice could not convert this document: {}", err));
        }
    };

    ctx.report(88.0, "Finalising");
    let found = find_output(&outdir, input, &expected_ext);
    let Some(found) = found else {
        let _ = fs::remove_dir_all(&work);
        let detail = stdout.trim();
        return Err(if detail.is_empty() {
            "LibreOffice reported success but produced no output file".to_string()
        } else {
            format!("LibreOffice produced no output file. {}", detail)
        });
    };

    write_output(&found, output)?;
    let _ = fs::remove_dir_all(&work);
    ctx.report(96.0, "Done");
    Ok(())
}

fn lo_filter(target_id: &str, source_family: Family) -> (String, String) {
    match target_id {
        "txt" => ("txt:Text".to_string(), "txt".to_string()),
        "html" => {
            let filter = match source_family {
                Family::Spreadsheet => "html:HTML (StarCalc)",
                _ => "html:HTML (StarWriter)",
            };
            (filter.to_string(), "html".to_string())
        }
        "csv" => (
            "csv:Text - txt - csv (StarCalc):44,34,76".to_string(),
            "csv".to_string(),
        ),
        "tsv" => (
            "csv:Text - txt - csv (StarCalc):9,34,76".to_string(),
            "csv".to_string(),
        ),
        "epub" => ("epub".to_string(), "epub".to_string()),
        other => (other.to_string(), other.to_string()),
    }
}

fn find_output(dir: &Path, input: &Path, expected_ext: &str) -> Option<PathBuf> {
    let input_stem = input.file_stem().map(|s| s.to_string_lossy().to_string());
    let entries = fs::read_dir(dir).ok()?;
    let mut candidates = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = path.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
        if name.starts_with(".~lock") || name.starts_with('.') {
            continue;
        }
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let stem_matches = input_stem
            .as_ref()
            .map(|s| {
                path.file_stem()
                    .map(|p| p.to_string_lossy().as_ref() == s.as_str())
                    .unwrap_or(false)
            })
            .unwrap_or(false);
        candidates.push((path, ext, stem_matches));
    }

    if let Some((path, _, _)) = candidates
        .iter()
        .find(|(_, ext, stem)| ext.as_str() == expected_ext && *stem)
    {
        return Some(path.clone());
    }
    if let Some((path, _, _)) = candidates
        .iter()
        .find(|(_, ext, _)| ext.as_str() == expected_ext)
    {
        return Some(path.clone());
    }
    candidates
        .iter()
        .find(|(_, _, stem)| *stem)
        .map(|(path, _, _)| path.clone())
        .or_else(|| candidates.first().map(|(path, _, _)| path.clone()))
}

fn write_output(from: &Path, to: &Path) -> Result<(), String> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Could not create output directory: {}", e))?;
    }
    if to.exists() {
        let _ = fs::remove_file(to);
    }
    match fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(_) => {
            fs::copy(from, to).map_err(|e| format!("Could not write output file: {}", e))?;
            let _ = fs::remove_file(from);
            Ok(())
        }
    }
}

/// Rasterise one PDF page, then let the image pipeline produce any image target.
pub fn pdf_to_image(
    ctx: &TaskCtx,
    input: &Path,
    output: &Path,
    target: &Spec,
    options: &Options,
) -> Result<(), String> {
    ctx.check()?;
    let page = options.page.unwrap_or(1).max(1);
    let dpi = options.dpi.unwrap_or(150).clamp(36, 600);

    // Vector targets are better served by Poppler/Ghostscript than by
    // rasterising through ImageMagick.
    if target.id == "svg" {
        let cairo = ctx
            .tools
            .pdftocairo
            .clone()
            .ok_or_else(|| "pdftocairo is not available".to_string())?;
        let args = vec![
            "-svg".to_string(),
            "-f".to_string(),
            page.to_string(),
            "-l".to_string(),
            page.to_string(),
            path_arg(input),
            path_arg(output),
        ];
        ctx.report(20.0, "Vectorising page");
        run(ctx, &cairo, &args, None)
            .map_err(|e| format!("Could not convert this PDF to SVG: {}", e))?;
        if !output.exists() {
            return Err("The PDF vector export produced no file".to_string());
        }
        ctx.report(96.0, "Done");
        return Ok(());
    }

    if target.id == "ps" || target.id == "eps" {
        let gs = ctx
            .tools
            .gs
            .clone()
            .ok_or_else(|| "Ghostscript is not available".to_string())?;
        let device = if target.id == "eps" { "eps2write" } else { "ps2write" };
        let args = vec![
            "-q".to_string(),
            "-dNOPAUSE".to_string(),
            "-dBATCH".to_string(),
            "-dSAFER".to_string(),
            format!("-sDEVICE={}", device),
            format!("-dFirstPage={}", page),
            format!("-dLastPage={}", page),
            format!("-sOutputFile={}", path_arg(output)),
            path_arg(input),
        ];
        ctx.report(20.0, "Writing PostScript");
        run(ctx, &gs, &args, None)
            .map_err(|e| format!("Could not convert this PDF to {}: {}", target.label, e))?;
        if !output.exists() {
            return Err("The PDF PostScript export produced no file".to_string());
        }
        ctx.report(96.0, "Done");
        return Ok(());
    }

    let program = ctx
        .tools
        .pdftoppm
        .clone()
        .ok_or_else(|| "Poppler's pdftoppm is not available".to_string())?;
    fs::create_dir_all(&ctx.temp)
        .map_err(|e| format!("Could not create scratch directory: {}", e))?;
    let prefix = ctx.temp.join(format!("pdf-{}", Uuid::new_v4().simple()));

    ctx.report(15.0, "Rasterising page");
    let args = vec![
        "-png".to_string(),
        "-singlefile".to_string(),
        "-f".to_string(),
        page.to_string(),
        "-l".to_string(),
        page.to_string(),
        "-r".to_string(),
        dpi.to_string(),
        path_arg(input),
        path_arg(&prefix),
    ];
    run(ctx, &program, &args, None)
        .map_err(|e| format!("Could not rasterise PDF page {}: {}", page, e))?;

    let png = if prefix.with_extension("png").exists() {
        prefix.with_extension("png")
    } else if prefix.exists() {
        prefix
    } else {
        return Err(format!("PDF page {} could not be rendered", page));
    };

    ctx.report(55.0, "Converting image");
    image::convert(ctx, &png, output, target, options)?;
    let _ = fs::remove_file(&png);
    ctx.report(96.0, "Done");
    Ok(())
}

/// Extract the text layer from a PDF.
pub fn pdf_to_text(ctx: &TaskCtx, input: &Path, output: &Path) -> Result<(), String> {
    ctx.check()?;
    let program = ctx
        .tools
        .pdftotext
        .clone()
        .ok_or_else(|| "pdftotext is not available".to_string())?;
    fs::create_dir_all(&ctx.temp)
        .map_err(|e| format!("Could not create scratch directory: {}", e))?;
    let temp = ctx.temp.join(format!("pdf-text-{}.txt", Uuid::new_v4().simple()));

    ctx.report(25.0, "Extracting text");
    let args = vec![
        "-layout".to_string(),
        "-enc".to_string(),
        "UTF-8".to_string(),
        path_arg(input),
        path_arg(&temp),
    ];
    run(ctx, &program, &args, None)
        .map_err(|e| format!("Could not extract text from this PDF: {}", e))?;

    if !temp.exists() {
        return Err("No text layer was produced for this PDF".to_string());
    }
    write_output(&temp, output)?;
    ctx.report(92.0, "Done");
    Ok(())
}

// ---------------------------------------------------------------------------
// EPUB reader
// ---------------------------------------------------------------------------

fn epub_to_html(ctx: &TaskCtx, input: &Path) -> Result<PathBuf, String> {
    let root = ctx.temp.join(format!("epub-{}", Uuid::new_v4().simple()));
    fs::create_dir_all(&root).map_err(|e| format!("Could not create EPUB workspace: {}", e))?;

    let file = File::open(input).map_err(|e| format!("Could not open EPUB: {}", e))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("Not a valid EPUB archive: {}", e))?;

    // Extract with the zip crate's traversal protection. We need real files
    // because the HTML we assemble may reference bundled images.
    for i in 0..archive.len() {
        let mut entry = match archive.by_index(i) {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        let Some(relative) = entry.enclosed_name() else {
            continue;
        };
        let destination = root.join(relative);
        if entry.is_dir() {
            let _ = fs::create_dir_all(&destination);
            continue;
        }
        if let Some(parent) = destination.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(mut out) = File::create(&destination) {
            let _ = std::io::copy(&mut entry, &mut out);
        }
    }

    let opf_path = locate_opf(&root);
    let mut chapters: Vec<PathBuf> = Vec::new();
    let mut opf_dir = root.clone();

    if let Some(opf) = opf_path.as_ref() {
        if let Some(parent) = opf.parent() {
            opf_dir = parent.to_path_buf();
        }
        if let Ok(opf_text) = fs::read_to_string(opf) {
            chapters = chapters_from_opf(&opf_text, &opf_dir);
        }
    }

    if chapters.is_empty() {
        chapters = html_files(&root);
    }
    chapters.sort();
    chapters.dedup();

    let mut body = String::new();
    for chapter in &chapters {
        if let Ok(text) = fs::read_to_string(chapter) {
            body.push_str(&extract_body(&text));
            body.push_str("\n<hr/>\n");
        }
    }

    if body.trim().is_empty() {
        return Err("This EPUB contains no readable chapters".to_string());
    }

    let html = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>{}</title>\
         <style>body{{font-family:serif;line-height:1.5;margin:2em;max-width:52em}}\
         img{{max-width:100%}}hr{{border:none;border-top:1px solid #ddd;margin:2em 0}}</style>\
         </head><body>{}</body></html>",
        xml_escape(
            &input
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "ebook".to_string())
        ),
        body
    );

    let out = root.join("combined.html");
    fs::write(&out, html).map_err(|e| format!("Could not assemble EPUB text: {}", e))?;
    let _ = opf_dir;
    Ok(out)
}

fn locate_opf(root: &Path) -> Option<PathBuf> {
    let container = root.join("META-INF/container.xml");
    if let Ok(text) = fs::read_to_string(&container) {
        if let Some(full_path) = attr(&text, "full-path") {
            let decoded = xml_unescape(&percent_decode(&full_path));
            let candidate = root.join(decoded.trim_start_matches('/'));
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    let mut found = Vec::new();
    collect_extensions(root, &mut found, &["opf"]);
    found.sort();
    found.into_iter().next()
}

fn chapters_from_opf(opf: &str, opf_dir: &Path) -> Vec<PathBuf> {
    let mut manifest: HashMap<String, (PathBuf, String)> = HashMap::new();
    for tag in opf.split('<').skip(1) {
        let trimmed = tag.trim_start();
        if !trimmed.starts_with("item")
            || trimmed
                .as_bytes()
                .get(4)
                .map(|b| !b.is_ascii_whitespace())
                .unwrap_or(true)
        {
            continue;
        }
        let tag = &tag[..tag.find('>').unwrap_or(tag.len())];
        let (Some(id), Some(href)) = (attr(tag, "id"), attr(tag, "href")) else {
            continue;
        };
        let media = attr(tag, "media-type").unwrap_or_default();
        let href = xml_unescape(&percent_decode(&href));
        let path = normalize_path(opf_dir, &href);
        manifest.insert(id, (path, media));
    }

    let mut chapters = Vec::new();
    for tag in opf.split('<').skip(1) {
        let trimmed = tag.trim_start();
        if !trimmed.starts_with("itemref")
            || trimmed
                .as_bytes()
                .get(7)
                .map(|b| !b.is_ascii_whitespace())
                .unwrap_or(true)
        {
            continue;
        }
        let tag = &tag[..tag.find('>').unwrap_or(tag.len())];
        if let Some(idref) = attr(tag, "idref") {
            if let Some((path, media)) = manifest.get(&idref) {
                if path.exists()
                    && (media.contains("html")
                        || path
                            .extension()
                            .map(|e| {
                                let e = e.to_string_lossy().to_lowercase();
                                e == "xhtml" || e == "html" || e == "htm"
                            })
                            .unwrap_or(false))
                {
                    chapters.push(path.clone());
                }
            }
        }
    }
    chapters
}

fn html_files(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    collect_extensions(root, &mut found, &["xhtml", "html", "htm"]);
    found.retain(|p| {
        p.file_name()
            .map(|n| {
                let n = n.to_string_lossy().to_lowercase();
                !n.starts_with("nav") && !n.starts_with("toc") && n != "combined.html"
            })
            .unwrap_or(true)
    });
    found
}

fn collect_extensions(dir: &Path, into: &mut Vec<PathBuf>, extensions: &[&str]) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_extensions(&path, into, extensions);
        } else if path
            .extension()
            .map(|e| {
                let e = e.to_string_lossy().to_lowercase();
                extensions.iter().any(|x| *x == e)
            })
            .unwrap_or(false)
        {
            into.push(path);
        }
    }
}

fn extract_body(html: &str) -> String {
    let lower = html.to_ascii_lowercase();
    let start = lower
        .find("<body")
        .and_then(|b| html[b..].find('>').map(|gt| b + gt + 1));
    let end = lower.rfind("</body>");
    match (start, end) {
        (Some(start), Some(end)) if end > start => html[start..end].to_string(),
        _ => html.to_string(),
    }
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let needle = format!("{}=", name);
    let mut from = 0;
    while let Some(found) = lower[from..].find(&needle) {
        let absolute = from + found;
        let boundary = absolute == 0
            || lower.as_bytes()[absolute - 1].is_ascii_whitespace()
            || lower.as_bytes()[absolute - 1] == b'<';
        if boundary {
            let mut start = absolute + needle.len();
            while lower.as_bytes().get(start).map(|b| b.is_ascii_whitespace()).unwrap_or(false) {
                start += 1;
            }
            let quote = match lower.as_bytes().get(start) {
                Some(b'"') => '"',
                Some(b'\'') => '\'',
                _ => {
                    from = absolute + needle.len();
                    continue;
                }
            };
            start += 1;
            if let Some(end) = tag[start..].find(quote) {
                return Some(tag[start..start + end].to_string());
            }
        }
        from = absolute + needle.len();
    }
    None
}

fn normalize_path(base: &Path, relative: &str) -> PathBuf {
    let mut out = base.to_path_buf();
    for component in Path::new(relative).components() {
        match component {
            Component::Normal(part) => out.push(part),
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            _ => {}
        }
    }
    out
}

fn percent_decode(input: &str) -> String {
    fn nibble(byte: u8) -> Option<u8> {
        match byte {
            b'0'..=b'9' => Some(byte - b'0'),
            b'a'..=b'f' => Some(byte - b'a' + 10),
            b'A'..=b'F' => Some(byte - b'A' + 10),
            _ => None,
        }
    }

    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(high), Some(low)) = (nibble(bytes[i + 1]), nibble(bytes[i + 2])) {
                out.push((high << 4) | low);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn xml_unescape(input: &str) -> String {
    input
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
}

fn xml_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Build a file:// URL with spaces and other reserved characters escaped so
/// LibreOffice accepts profile paths containing whitespace.
fn file_url(path: &Path) -> String {
    let raw = path.to_string_lossy();
    let mut out = String::from("file://");
    for byte in raw.bytes() {
        match byte {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'/'
            | b'-'
            | b'_'
            | b'.'
            | b'~' => out.push(byte as char),
            _ => out.push_str(&format!("%{:02X}", byte)),
        }
    }
    out
}
