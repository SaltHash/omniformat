use std::collections::HashMap;
use std::sync::OnceLock;

use crate::tools::Tools;
use crate::types::{Catalog, FormatInfo};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    Image,
    Vector,
    Video,
    Audio,
    Document,
    Spreadsheet,
    Presentation,
    Pdf,
    Archive,
    Subtitle,
}

impl Family {
    pub fn id(self) -> &'static str {
        match self {
            Family::Image => "image",
            Family::Vector => "vector",
            Family::Video => "video",
            Family::Audio => "audio",
            Family::Document => "document",
            Family::Spreadsheet => "spreadsheet",
            Family::Presentation => "presentation",
            Family::Pdf => "pdf",
            Family::Archive => "archive",
            Family::Subtitle => "subtitle",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Family::Image => "Image",
            Family::Vector => "Vector",
            Family::Video => "Video",
            Family::Audio => "Audio",
            Family::Document => "Document",
            Family::Spreadsheet => "Spreadsheet",
            Family::Presentation => "Presentation",
            Family::Pdf => "PDF",
            Family::Archive => "Archive",
            Family::Subtitle => "Subtitle",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Engine {
    Image,
    Media,
    Document,
    Pdf,
    Archive,
    Subtitle,
}

#[derive(Clone, Copy, Debug)]
pub struct Spec {
    pub id: &'static str,
    pub label: &'static str,
    pub family: Family,
    pub extensions: &'static [&'static str],
    pub mime: &'static str,
    pub note: &'static str,
    pub lossless: bool,
    pub magick: Option<&'static str>,
    pub ff_muxer: Option<&'static str>,
    pub ff_video: Option<&'static str>,
    pub ff_audio: Option<&'static str>,
    pub read_only: bool,
    pub write_only: bool,
    pub engine: Engine,
}

fn image(
    id: &'static str,
    label: &'static str,
    magick: &'static str,
    extensions: &'static [&'static str],
    mime: &'static str,
    note: &'static str,
    lossless: bool,
) -> Spec {
    Spec {
        id,
        label,
        family: Family::Image,
        extensions,
        mime,
        note,
        lossless,
        magick: Some(magick),
        ff_muxer: None,
        ff_video: None,
        ff_audio: None,
        read_only: false,
        write_only: false,
        engine: Engine::Image,
    }
}

fn raw(id: &'static str, label: &'static str, extensions: &'static [&'static str], note: &'static str) -> Spec {
    Spec {
        id,
        label,
        family: Family::Image,
        extensions,
        mime: "image/x-raw",
        note,
        lossless: true,
        magick: Some("DNG"),
        ff_muxer: None,
        ff_video: None,
        ff_audio: None,
        read_only: true,
        write_only: false,
        engine: Engine::Image,
    }
}

fn vector(
    id: &'static str,
    label: &'static str,
    magick: &'static str,
    extensions: &'static [&'static str],
    mime: &'static str,
    note: &'static str,
    read_only: bool,
) -> Spec {
    Spec {
        id,
        label,
        family: Family::Vector,
        extensions,
        mime,
        note,
        lossless: true,
        magick: Some(magick),
        ff_muxer: None,
        ff_video: None,
        ff_audio: None,
        read_only,
        write_only: false,
        engine: Engine::Image,
    }
}

fn media(
    id: &'static str,
    label: &'static str,
    family: Family,
    extensions: &'static [&'static str],
    mime: &'static str,
    muxer: &'static str,
    video: Option<&'static str>,
    audio: Option<&'static str>,
    note: &'static str,
    lossless: bool,
) -> Spec {
    Spec {
        id,
        label,
        family,
        extensions,
        mime,
        note,
        lossless,
        magick: None,
        ff_muxer: Some(muxer),
        ff_video: video,
        ff_audio: audio,
        read_only: false,
        write_only: false,
        engine: Engine::Media,
    }
}

fn doc(
    id: &'static str,
    label: &'static str,
    family: Family,
    extensions: &'static [&'static str],
    mime: &'static str,
    note: &'static str,
    read_only: bool,
) -> Spec {
    Spec {
        id,
        label,
        family,
        extensions,
        mime,
        note,
        lossless: true,
        magick: None,
        ff_muxer: None,
        ff_video: None,
        ff_audio: None,
        read_only,
        write_only: false,
        engine: Engine::Document,
    }
}

fn archive(
    id: &'static str,
    label: &'static str,
    extensions: &'static [&'static str],
    note: &'static str,
    read_only: bool,
) -> Spec {
    Spec {
        id,
        label,
        family: Family::Archive,
        extensions,
        mime: "application/octet-stream",
        note,
        lossless: true,
        magick: None,
        ff_muxer: None,
        ff_video: None,
        ff_audio: None,
        read_only,
        write_only: false,
        engine: Engine::Archive,
    }
}

fn subtitle(
    id: &'static str,
    label: &'static str,
    extensions: &'static [&'static str],
    muxer: &'static str,
    read_only: bool,
) -> Spec {
    Spec {
        id,
        label,
        family: Family::Subtitle,
        extensions,
        mime: "text/vtt",
        note: if read_only { "Read only" } else { "Text" },
        lossless: true,
        magick: None,
        ff_muxer: Some(muxer),
        ff_video: None,
        ff_audio: None,
        read_only,
        write_only: false,
        engine: Engine::Subtitle,
    }
}

fn build() -> Vec<Spec> {
    let mut v = Vec::new();

    // ---- Images -----------------------------------------------------------
    v.extend([
        image("png", "PNG", "PNG", &["png"], "image/png", "Lossless", true),
        image("jpg", "JPEG", "JPEG", &["jpg", "jpeg", "jpe"], "image/jpeg", "Universal", false),
        image("webp", "WebP", "WEBP", &["webp"], "image/webp", "Modern", false),
        image("gif", "GIF", "GIF", &["gif"], "image/gif", "Animated", true),
        image("bmp", "BMP", "BMP", &["bmp"], "image/bmp", "Legacy", true),
        image("tiff", "TIFF", "TIFF", &["tiff", "tif"], "image/tiff", "Print", true),
        image("tga", "TGA", "TGA", &["tga"], "image/x-tga", "Game", true),
        image("ico", "ICO", "ICO", &["ico"], "image/x-icon", "Icon", true),
        image("psd", "PSD", "PSD", &["psd"], "image/vnd.adobe.photoshop", "Layered", true),
        image("psb", "PSB", "PSD", &["psb"], "image/vnd.adobe.photoshop", "Large PSD", true),
        image("xcf", "XCF", "XCF", &["xcf"], "image/x-xcf", "GIMP", true),
        image("dds", "DDS", "DDS", &["dds"], "image/vnd-ms.dds", "Texture", true),
        image("exr", "OpenEXR", "EXR", &["exr"], "image/x-exr", "HDR", true),
        image("hdr", "Radiance", "HDR", &["hdr"], "image/vnd.radiance", "HDR", true),
        image("pcx", "PCX", "PCX", &["pcx"], "image/x-pcx", "Legacy", true),
        image("pnm", "PNM", "PNM", &["pnm"], "image/x-portable-anymap", "Raw", true),
        image("ppm", "PPM", "PPM", &["ppm"], "image/x-portable-pixmap", "Raw", true),
        image("pgm", "PGM", "PGM", &["pgm"], "image/x-portable-graymap", "Raw", true),
        image("pbm", "PBM", "PBM", &["pbm"], "image/x-portable-bitmap", "Mono", true),
        image("pam", "PAM", "PAM", &["pam"], "image/x-portable-arbitrarymap", "Raw", true),
        image("pfm", "PFM", "PFM", &["pfm"], "image/x-portable-floatmap", "Float", true),
        image("qoi", "QOI", "QOI", &["qoi"], "image/qoi", "Fast", true),
        image("jp2", "JPEG 2000", "JP2", &["jp2"], "image/jp2", "Archival", false),
        image("j2k", "JPEG 2000 codestream", "J2K", &["j2k", "j2c"], "image/jp2", "Codestream", false),
        image("jxl", "JPEG XL", "JXL", &["jxl"], "image/jxl", "Next-gen", true),
        image("fits", "FITS", "FITS", &["fits", "fts"], "image/fits", "Astronomy", true),
        image("dpx", "DPX", "DPX", &["dpx"], "image/x-dpx", "Film", false),
        image("cin", "Cineon", "CIN", &["cin"], "image/x-cineon", "Film", false),
        image("sgi", "SGI", "SGI", &["sgi", "rgb"], "image/x-sgi", "Legacy", true),
        image("ras", "Sun Raster", "SUN", &["ras"], "image/x-sun-raster", "Legacy", true),
        image("pict", "PICT", "PICT", &["pict", "pct"], "image/x-pict", "Mac", true),
        image("xwd", "XWD", "XWD", &["xwd"], "image/x-xwindowdump", "X11", true),
        image("viff", "VIFF", "VIFF", &["viff"], "image/x-viff", "KHOROS", true),
        image("miff", "MIFF", "MIFF", &["miff"], "image/x-miff", "ImageMagick", true),
        image("xbm", "XBM", "XBM", &["xbm"], "image/x-xbitmap", "Mono", true),
        image("xpm", "XPM", "XPM", &["xpm"], "image/x-xpixmap", "X11", true),
        image("apng", "APNG", "APNG", &["apng"], "image/apng", "Animated", true),
        image("ora", "OpenRaster", "ORA", &["ora"], "image/openraster", "Layered", true),
        image("dcm", "DICOM", "DCM", &["dcm"], "application/dicom", "Medical", true),
    ]);

    for (id, label, exts, note) in [
        ("cr2", "Canon RAW", &["cr2"][..], "Canon"),
        ("cr3", "Canon RAW 3", &["cr3"][..], "Canon"),
        ("nef", "Nikon RAW", &["nef"][..], "Nikon"),
        ("arw", "Sony RAW", &["arw"][..], "Sony"),
        ("dng", "DNG", &["dng"][..], "Adobe"),
        ("raf", "Fujifilm RAW", &["raf"][..], "Fujifilm"),
        ("orf", "Olympus RAW", &["orf"][..], "Olympus"),
        ("rw2", "Panasonic RAW", &["rw2"][..], "Panasonic"),
        ("pef", "Pentax RAW", &["pef"][..], "Pentax"),
        ("srw", "Samsung RAW", &["srw"][..], "Samsung"),
        ("3fr", "Hasselblad RAW", &["3fr"][..], "Hasselblad"),
        ("erf", "Epson RAW", &["erf"][..], "Epson"),
        ("mrw", "Minolta RAW", &["mrw"][..], "Minolta"),
        ("nrw", "Nikon NRW", &["nrw"][..], "Nikon"),
        ("rwl", "Leica RAW", &["rwl"][..], "Leica"),
        ("sr2", "Sony SR2", &["sr2"][..], "Sony"),
        ("srf", "Sony SRF", &["srf"][..], "Sony"),
        ("kdc", "Kodak RAW", &["kdc"][..], "Kodak"),
        ("mef", "Mamiya RAW", &["mef"][..], "Mamiya"),
        ("iiq", "Phase One IIQ", &["iiq"][..], "Phase One"),
    ] {
        v.push(raw(id, label, exts, note));
    }

    // ---- Vector -----------------------------------------------------------
    v.extend([
        vector("svg", "SVG", "SVG", &["svg"], "image/svg+xml", "Vector", false),
        vector("svgz", "SVGZ", "SVGZ", &["svgz"], "image/svg+xml", "Compressed", false),
        vector("eps", "EPS", "EPS", &["eps"], "application/postscript", "Vector", false),
        vector("ps", "PostScript", "PS", &["ps"], "application/postscript", "Vector", false),
        vector("ai", "Illustrator", "AI", &["ai"], "application/postscript", "Read only", true),
    ]);

    // ---- Video ------------------------------------------------------------
    v.extend([
        media("mp4", "MP4", Family::Video, &["mp4"], "video/mp4", "mp4", Some("h264"), Some("aac"), "Universal", false),
        media("mkv", "Matroska", Family::Video, &["mkv"], "video/x-matroska", "matroska", Some("h264"), Some("aac"), "Flexible", false),
        media("webm", "WebM", Family::Video, &["webm"], "video/webm", "webm", Some("vp9"), Some("opus"), "Web", false),
        media("mov", "QuickTime", Family::Video, &["mov"], "video/quicktime", "mov", Some("h264"), Some("aac"), "Apple", false),
        media("m4v", "M4V", Family::Video, &["m4v"], "video/x-m4v", "mp4", Some("h264"), Some("aac"), "Apple TV", false),
        media("avi", "AVI", Family::Video, &["avi"], "video/x-msvideo", "avi", Some("mpeg4"), Some("mp3"), "Legacy", false),
        media("mpg", "MPEG", Family::Video, &["mpg", "mpeg"], "video/mpeg", "mpeg", Some("mpeg2video"), Some("mp2"), "Legacy", false),
        media("ts", "MPEG-TS", Family::Video, &["ts"], "video/mp2t", "mpegts", Some("h264"), Some("aac"), "Broadcast", false),
        media("m2ts", "M2TS", Family::Video, &["m2ts", "mts"], "video/mp2t", "mpegts", Some("h264"), Some("ac3"), "Blu-ray", false),
        media("flv", "FLV", Family::Video, &["flv"], "video/x-flv", "flv", Some("h264"), Some("aac"), "Flash", false),
        media("wmv", "WMV", Family::Video, &["wmv"], "video/x-ms-wmv", "asf", Some("wmv2"), Some("wmav2"), "Windows", false),
        media("3gp", "3GP", Family::Video, &["3gp"], "video/3gpp", "3gp", Some("h264"), Some("aac"), "Mobile", false),
        media("ogv", "OGV", Family::Video, &["ogv"], "video/ogg", "ogg", Some("theora"), Some("vorbis"), "Open", false),
        media("vob", "VOB", Family::Video, &["vob"], "video/dvd", "mpeg", Some("mpeg2video"), Some("ac3"), "DVD", false),
        media("mxf", "MXF", Family::Video, &["mxf"], "video/mxf", "mxf", Some("mpeg2video"), Some("pcm_s16le"), "Broadcast", false),
        media("nut", "NUT", Family::Video, &["nut"], "video/x-nut", "nut", Some("h264"), Some("aac"), "FFmpeg", false),
        media("ivf", "IVF", Family::Video, &["ivf"], "video/x-ivf", "ivf", Some("vp9"), None, "Raw VP9", false),
        media("dv", "DV", Family::Video, &["dv"], "video/dv", "dv", Some("dvvideo"), Some("pcm_s16le"), "Camcorder", false),
    ]);

    // ---- Audio ------------------------------------------------------------
    v.extend([
        media("mp3", "MP3", Family::Audio, &["mp3"], "audio/mpeg", "mp3", None, Some("mp3"), "Universal", false),
        media("wav", "WAV", Family::Audio, &["wav"], "audio/wav", "wav", None, Some("pcm_s16le"), "Lossless PCM", true),
        media("flac", "FLAC", Family::Audio, &["flac"], "audio/flac", "flac", None, Some("flac"), "Lossless", true),
        media("aac", "AAC", Family::Audio, &["aac"], "audio/aac", "adts", None, Some("aac"), "Efficient", false),
        media("m4a", "M4A", Family::Audio, &["m4a"], "audio/mp4", "ipod", None, Some("aac"), "Apple", false),
        media("m4b", "M4B", Family::Audio, &["m4b"], "audio/mp4", "ipod", None, Some("aac"), "Audiobook", false),
        media("ogg", "OGG", Family::Audio, &["ogg"], "audio/ogg", "ogg", None, Some("vorbis"), "Open", false),
        media("oga", "OGA", Family::Audio, &["oga"], "audio/ogg", "oga", None, Some("vorbis"), "Open", false),
        media("opus", "Opus", Family::Audio, &["opus"], "audio/opus", "opus", None, Some("opus"), "Speech", false),
        media("wma", "WMA", Family::Audio, &["wma"], "audio/x-ms-wma", "asf", None, Some("wmav2"), "Windows", false),
        media("aiff", "AIFF", Family::Audio, &["aiff", "aif"], "audio/aiff", "aiff", None, Some("pcm_s16be"), "Apple PCM", true),
        media("ac3", "AC-3", Family::Audio, &["ac3"], "audio/ac3", "ac3", None, Some("ac3"), "Surround", false),
        media("eac3", "E-AC-3", Family::Audio, &["eac3"], "audio/eac3", "eac3", None, Some("eac3"), "Surround", false),
        media("amr", "AMR", Family::Audio, &["amr"], "audio/amr", "amr", None, Some("amr_nb"), "Voice", false),
        media("au", "AU", Family::Audio, &["au"], "audio/basic", "au", None, Some("pcm_s16be"), "Legacy", true),
        media("caf", "CAF", Family::Audio, &["caf"], "audio/x-caf", "caf", None, Some("pcm_s16le"), "Apple", true),
        media("mka", "MKA", Family::Audio, &["mka"], "audio/x-matroska", "matroska", None, Some("flac"), "Container", true),
        media("mp2", "MP2", Family::Audio, &["mp2"], "audio/mpeg", "mp2", None, Some("mp2"), "Broadcast", false),
        media("wv", "WavPack", Family::Audio, &["wv"], "audio/x-wavpack", "wv", None, Some("wavpack"), "Lossless", true),
        media("tta", "TTA", Family::Audio, &["tta"], "audio/x-tta", "tta", None, Some("tta"), "Lossless", true),
    ]);

    // ---- Documents --------------------------------------------------------
    v.extend([
        doc("docx", "DOCX", Family::Document, &["docx"], "application/vnd.openxmlformats-officedocument.wordprocessingml.document", "Word", false),
        doc("doc", "DOC", Family::Document, &["doc"], "application/msword", "Word legacy", false),
        doc("odt", "ODT", Family::Document, &["odt"], "application/vnd.oasis.opendocument.text", "OpenDocument", false),
        doc("ott", "OTT", Family::Document, &["ott"], "application/vnd.oasis.opendocument.text-template", "Template", false),
        doc("rtf", "RTF", Family::Document, &["rtf"], "application/rtf", "Rich text", false),
        doc("txt", "Text", Family::Document, &["txt"], "text/plain", "Plain", false),
        doc("html", "HTML", Family::Document, &["html", "htm"], "text/html", "Web", false),
        doc("epub", "EPUB", Family::Document, &["epub"], "application/epub+zip", "Ebook", false),
    ]);

    // ---- Spreadsheets -----------------------------------------------------
    v.extend([
        doc("xlsx", "XLSX", Family::Spreadsheet, &["xlsx"], "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", "Excel", false),
        doc("xls", "XLS", Family::Spreadsheet, &["xls"], "application/vnd.ms-excel", "Excel legacy", false),
        doc("ods", "ODS", Family::Spreadsheet, &["ods"], "application/vnd.oasis.opendocument.spreadsheet", "OpenDocument", false),
        doc("csv", "CSV", Family::Spreadsheet, &["csv"], "text/csv", "Data", false),
        doc("tsv", "TSV", Family::Spreadsheet, &["tsv"], "text/tab-separated-values", "Data", false),
    ]);

    // ---- Presentations ----------------------------------------------------
    v.extend([
        doc("pptx", "PPTX", Family::Presentation, &["pptx"], "application/vnd.openxmlformats-officedocument.presentationml.presentation", "Slides", false),
        doc("ppt", "PPT", Family::Presentation, &["ppt"], "application/vnd.ms-powerpoint", "Slides legacy", false),
        doc("odp", "ODP", Family::Presentation, &["odp"], "application/vnd.oasis.opendocument.presentation", "OpenDocument", false),
        doc("otp", "OTP", Family::Presentation, &["otp"], "application/vnd.oasis.opendocument.presentation-template", "Template", false),
    ]);

    // ---- PDF --------------------------------------------------------------
    v.push(Spec {
        id: "pdf",
        label: "PDF",
        family: Family::Pdf,
        extensions: &["pdf"],
        mime: "application/pdf",
        note: "Portable",
        lossless: true,
        magick: Some("PDF"),
        ff_muxer: None,
        ff_video: None,
        ff_audio: None,
        read_only: false,
        write_only: false,
        engine: Engine::Pdf,
    });

    // ---- Archives ---------------------------------------------------------
    v.extend([
        archive("zip", "ZIP", &["zip"], "Universal", false),
        archive("7z", "7Z", &["7z"], "Compact", false),
        archive("tar", "TAR", &["tar"], "Unix", false),
        archive("tar.gz", "TAR.GZ", &["tar.gz", "tgz"], "Compressed", false),
        archive("tar.bz2", "TAR.BZ2", &["tar.bz2", "tbz2"], "Compressed", false),
        archive("tar.xz", "TAR.XZ", &["tar.xz", "txz"], "Compressed", false),
        archive("tar.zst", "TAR.ZST", &["tar.zst", "tzst"], "Fast", false),
        archive("rar", "RAR", &["rar"], "Read only", true),
        archive("iso", "ISO", &["iso"], "Disk image", true),
        archive("cab", "CAB", &["cab"], "Windows", true),
        archive("wim", "WIM", &["wim"], "Windows image", true),
        archive("deb", "DEB", &["deb"], "Debian", true),
        archive("rpm", "RPM", &["rpm"], "Fedora", true),
        archive("cpio", "CPIO", &["cpio"], "Unix", true),
        archive("lzh", "LZH", &["lzh", "lha"], "Legacy", true),
        archive("arj", "ARJ", &["arj"], "Legacy", true),
        archive("chm", "CHM", &["chm"], "Help", true),
        archive("dmg", "DMG", &["dmg"], "macOS", true),
    ]);

    // ---- Subtitles --------------------------------------------------------
    v.extend([
        subtitle("srt", "SubRip", &["srt"], "srt", false),
        subtitle("vtt", "WebVTT", &["vtt"], "webvtt", false),
        subtitle("ass", "ASS", &["ass"], "ass", false),
        subtitle("ssa", "SSA", &["ssa"], "ass", false),
        subtitle("ttml", "TTML", &["ttml"], "ttml", false),
        subtitle("sub", "MicroDVD", &["sub"], "microdvd", true),
        subtitle("sbv", "SBV", &["sbv"], "srt", true),
        subtitle("smi", "SAMI", &["smi"], "sami", true),
    ]);

    // DV output is lossy in a very specific way (fixed frame size, 48 kHz
    // PCM) and FFmpeg produces sync warnings on ordinary sources. Keep DV as
    // a readable input rather than advertising a brittle output.
    if let Some(dv) = v.iter_mut().find(|s| s.id == "dv") {
        dv.read_only = true;
    }
    // AMR output depends on fragile encoder/muxer frame alignment in bare
    // FFmpeg builds; keep it readable but do not offer it as a target.
    if let Some(amr) = v.iter_mut().find(|s| s.id == "amr") {
        amr.read_only = true;
    }

    v
}

pub fn specs() -> &'static [Spec] {
    static SPECS: OnceLock<Vec<Spec>> = OnceLock::new();
    SPECS.get_or_init(build)
}

pub fn lookup(id: &str) -> Option<&'static Spec> {
    specs().iter().find(|s| s.id.eq_ignore_ascii_case(id))
}

pub fn from_path(path: &std::path::Path) -> Option<&'static Spec> {
    let name = path.file_name()?.to_string_lossy().to_lowercase();
    let mut best: Option<(&Spec, usize)> = None;
    for spec in specs() {
        for ext in spec.extensions {
            let suffix = format!(".{}", ext.to_lowercase());
            if name.ends_with(&suffix) {
                let len = suffix.len();
                if best.map(|(_, l)| len > l).unwrap_or(true) {
                    best = Some((spec, len));
                }
            }
        }
    }
    best.map(|(s, _)| s)
}

pub fn from_mime(mime: &str) -> Option<&'static Spec> {
    let mime = mime.trim().to_lowercase();
    specs().iter().find(|s| s.mime.eq_ignore_ascii_case(&mime))
}

pub fn from_extension(ext: &str) -> Option<&'static Spec> {
    let ext = ext.trim_start_matches('.').to_lowercase();
    specs().iter().find(|s| s.extensions.iter().any(|e| e.eq_ignore_ascii_case(&ext)))
}

pub fn family_of(id: &str) -> Family {
    lookup(id).map(|s| s.family).unwrap_or(Family::Document)
}

struct Availability {
    read: bool,
    write: bool,
}

fn available(spec: &Spec, tools: &Tools) -> Availability {
    let mut a = Availability { read: !spec.write_only, write: !spec.read_only };

    match spec.engine {
        Engine::Image => {
            let (r, w) = spec.magick.map(|m| tools.magick_has(m)).unwrap_or((false, false));
            a.read &= r && tools.has_images();
            a.write &= w && tools.has_images();
        }
        Engine::Media => {
            let mux = spec.ff_muxer.map(|m| tools.muxer(m)).unwrap_or(false);
            let v = spec.ff_video.map(|c| tools.has_video_encoder(c)).unwrap_or(true);
            let aud = spec.ff_audio.map(|c| tools.has_audio_encoder(c)).unwrap_or(true);
            a.read &= tools.has_video();
            a.write &= tools.has_video() && mux && v && aud;
        }
        Engine::Document => {
            a.read &= tools.has_documents();
            a.write &= tools.has_documents();
        }
        Engine::Pdf => {
            a.read &= tools.has_pdf_read() || tools.pdfinfo.is_some();
            a.write &= tools.has_documents() || tools.has_images() || tools.gs.is_some();
        }
        Engine::Archive => {
            let seven = tools.sevenzip.is_some();
            let tar = tools.tar.is_some();
            let is_tar = spec.id.starts_with("tar");
            a.read &= if is_tar { seven || tar } else { seven };
            a.write &= if is_tar {
                tar && match spec.id {
                    "tar.gz" => tools.gzip.is_some(),
                    "tar.bz2" => tools.bzip2.is_some(),
                    "tar.xz" => tools.xz.is_some(),
                    "tar.zst" => tools.zstd.is_some(),
                    _ => true,
                }
            } else {
                seven
            };
            if spec.read_only {
                // 7-Zip's build here exposes most of these; RAR requires an
                // external codec, so only advertise it when 7-Zip lists it.
                a.read &= seven && (spec.id != "rar" || tools.sevenzip_formats.contains("rar"));
            }
        }
        Engine::Subtitle => {
            let mux = spec.ff_muxer.map(|m| tools.muxer(m)).unwrap_or(false);
            a.read &= tools.has_video();
            a.write &= tools.has_video() && mux;
        }
    }

    if spec.read_only {
        a.write = false;
    }
    if spec.write_only {
        a.read = false;
    }
    a
}

fn writable(tools: &Tools, family: Family) -> Vec<&'static Spec> {
    specs()
        .iter()
        .filter(|s| s.family == family && available(s, tools).write)
        .collect()
}

fn readables(tools: &Tools, family: Family) -> Vec<&'static Spec> {
    specs()
        .iter()
        .filter(|s| s.family == family && available(s, tools).read)
        .collect()
}

fn ids(specs: &[&'static Spec]) -> Vec<String> {
    specs.iter().map(|s| s.id.to_string()).collect()
}

fn push_unique(out: &mut Vec<String>, ids: impl IntoIterator<Item = String>) {
    for id in ids {
        if !out.contains(&id) {
            out.push(id);
        }
    }
}

pub fn targets_for(source: &Spec, tools: &Tools) -> Vec<String> {
    let mut out = Vec::new();
    let images = || writable(tools, Family::Image);

    match source.family {
        Family::Image => {
            push_unique(&mut out, ids(&images()));
            push_unique(&mut out, vec!["pdf".into()]);
            for id in ["mp4", "webm", "mkv"] {
                if let Some(spec) = lookup(id) {
                    if available(spec, tools).write {
                        out.push(id.into());
                    }
                }
            }
            out.retain(|id| id != "dcm");
        }
        Family::Vector => {
            push_unique(&mut out, ids(&images()));
            push_unique(&mut out, vec!["pdf".into()]);
            for id in ["eps", "ps", "svg"] {
                if let Some(spec) = lookup(id) {
                    if available(spec, tools).write {
                        out.push(id.into());
                    }
                }
            }
        }
        Family::Video => {
            push_unique(&mut out, ids(&writable(tools, Family::Video)));
            push_unique(&mut out, ids(&writable(tools, Family::Audio)));
            push_unique(&mut out, ids(&images()));
            out.retain(|id| id != "dcm");
        }
        Family::Audio => {
            push_unique(&mut out, ids(&writable(tools, Family::Audio)));
            for id in ["mp4", "webm", "mkv"] {
                if let Some(spec) = lookup(id) {
                    if available(spec, tools).write {
                        out.push(id.into());
                    }
                }
            }
            if available(lookup("png").unwrap(), tools).write {
                out.push("png".into());
            }
        }
        Family::Document => {
            push_unique(&mut out, ids(&writable(tools, Family::Document)));
            push_unique(&mut out, vec!["pdf".into()]);
            let is_html = source.id == "html";
            let is_epub = source.id == "epub";
            if is_html || is_epub {
                out.retain(|id| {
                    !matches!(id.as_str(), "doc" | "docx" | "odt" | "ott" | "rtf" | "epub")
                });
            }
            if available(lookup("png").unwrap(), tools).write {
                out.push("png".into());
            }
            if !out.contains(&"txt".to_string()) {
                out.push("txt".into());
            }
        }
        Family::Spreadsheet => {
            push_unique(&mut out, ids(&writable(tools, Family::Spreadsheet)));
            push_unique(&mut out, vec!["pdf".into(), "csv".into(), "tsv".into(), "html".into()]);
            if available(lookup("png").unwrap(), tools).write {
                out.push("png".into());
            }
        }
        Family::Presentation => {
            push_unique(&mut out, ids(&writable(tools, Family::Presentation)));
            push_unique(&mut out, vec!["pdf".into()]);
            if available(lookup("png").unwrap(), tools).write {
                out.push("png".into());
            }
        }
        Family::Pdf => {
            push_unique(&mut out, ids(&images()));
            out.retain(|id| id != "dcm");
            push_unique(&mut out, vec!["txt".into()]);
            if tools.pdftocairo.is_some() {
                out.push("svg".into());
            }
            if tools.gs.is_some() {
                out.push("ps".into());
                out.push("eps".into());
            }
        }
        Family::Archive => {
            push_unique(&mut out, ids(&writable(tools, Family::Archive)));
        }
        Family::Subtitle => {
            push_unique(&mut out, ids(&writable(tools, Family::Subtitle)));
            out.push("txt".into());
        }
    }

    // Keep only formats we can actually write, and de-duplicate while keeping order.
    out.retain(|id| lookup(id).map(|s| available(s, tools).write).unwrap_or(false));
    out
}

/// The short list of outputs worth offering for a source type. Curated by
/// what people actually reach for (JPEG/WebP/ICO from a PNG, MP4/WebM/GIF
/// from a video) instead of dumping every supported container at the top of
/// the picker. Filtered against the current toolchain and capped at five.
pub fn suggestions(source: &Spec, tools: &Tools) -> Vec<String> {
    let ranked: &[&str] = match source.id {
        "png" => &["jpg", "webp", "ico", "pdf", "tiff"],
        "jpg" => &["png", "webp", "pdf", "ico", "tiff"],
        "webp" => &["jpg", "png", "ico", "pdf", "tiff"],
        "gif" => &["webp", "mp4", "png", "jpg", "ico"],
        "apng" => &["webp", "gif", "png", "mp4", "jpg"],
        "ico" => &["png", "jpg", "webp", "bmp", "tiff"],
        "bmp" => &["png", "jpg", "webp", "pdf", "tiff"],
        "tiff" => &["png", "jpg", "webp", "pdf", "bmp"],
        "exr" | "hdr" | "dpx" | "cin" | "fits" => {
            &["png", "jpg", "tiff", "webp", "pdf"]
        }
        "xbm" | "xpm" => &["png", "jpg", "ico", "webp", "bmp"],
        "psd" | "psb" | "xcf" | "ora" => &["png", "jpg", "webp", "pdf", "tiff"],
        "cr2" | "cr3" | "nef" | "arw" | "dng" | "raf" | "orf" | "rw2" | "pef" | "srw"
        | "3fr" | "erf" | "mrw" | "nrw" | "rwl" | "sr2" | "srf" | "kdc" | "mef" | "iiq" => {
            &["jpg", "png", "tiff", "webp", "pdf"]
        }

        "svg" | "svgz" => &["png", "pdf", "jpg", "eps", "webp"],
        "ai" => &["pdf", "png", "jpg", "svg", "eps"],
        "eps" | "ps" => &["pdf", "png", "jpg", "svg", "webp"],

        "mp4" => &["webm", "mkv", "mov", "gif", "mp3"],
        "mkv" => &["mp4", "webm", "mov", "gif", "mp3"],
        "mov" => &["mp4", "webm", "mkv", "gif", "mp3"],
        "webm" => &["mp4", "mkv", "gif", "mp3", "mov"],
        "m4v" => &["mp4", "mov", "mkv", "webm", "mp3"],
        "avi" => &["mp4", "mkv", "webm", "gif", "mp3"],

        "mp3" => &["wav", "flac", "m4a", "ogg", "opus"],
        "wav" => &["mp3", "flac", "m4a", "ogg", "opus"],
        "flac" => &["mp3", "wav", "m4a", "ogg", "opus"],
        "m4a" | "m4b" => &["mp3", "wav", "flac", "ogg", "opus"],
        "ogg" | "oga" => &["mp3", "wav", "flac", "m4a", "opus"],
        "opus" => &["mp3", "wav", "flac", "m4a", "ogg"],
        "aac" => &["mp3", "m4a", "wav", "flac", "ogg"],
        "aiff" => &["wav", "mp3", "flac", "m4a", "ogg"],

        "txt" => &["pdf", "docx", "odt", "rtf", "html"],
        "docx" => &["pdf", "txt", "odt", "rtf", "html"],
        "doc" => &["docx", "pdf", "txt", "odt", "html"],
        "odt" | "ott" => &["pdf", "docx", "txt", "rtf", "html"],
        "rtf" => &["pdf", "docx", "txt", "odt", "html"],
        "html" | "htm" | "epub" => &["pdf", "txt", "png"],

        "xlsx" | "xls" | "ods" => &["pdf", "csv", "xlsx", "tsv", "html"],
        "csv" => &["xlsx", "pdf", "ods", "tsv", "html"],
        "tsv" => &["xlsx", "pdf", "ods", "csv", "html"],

        "pptx" | "ppt" | "odp" | "otp" => &["pdf", "pptx", "odp", "png", "otp"],

        "zip" => &["7z", "tar.gz", "tar.xz", "tar", "tar.zst"],
        "7z" => &["zip", "tar.gz", "tar.xz", "tar", "tar.zst"],
        "tar" => &["zip", "7z", "tar.gz", "tar.xz", "tar.zst"],
        "tar.gz" => &["zip", "7z", "tar.xz", "tar.zst", "tar"],

        "srt" => &["vtt", "ass", "txt", "ssa", "ttml"],
        "vtt" => &["srt", "ass", "txt", "ttml", "ssa"],
        "ass" | "ssa" => &["srt", "vtt", "txt", "ttml", "ssa"],
        "ttml" => &["srt", "vtt", "ass", "txt", "ssa"],

        _ => match source.family {
            Family::Image => &["png", "jpg", "webp", "pdf", "tiff"],
            Family::Vector => &["png", "pdf", "jpg", "svg", "eps"],
            Family::Video => &["mp4", "webm", "mkv", "gif", "mp3"],
            Family::Audio => &["mp3", "wav", "flac", "m4a", "ogg"],
            Family::Document => &["pdf", "docx", "txt", "odt", "html"],
            Family::Spreadsheet => &["pdf", "xlsx", "csv", "ods", "html"],
            Family::Presentation => &["pdf", "pptx", "odp", "png"],
            Family::Pdf => &["png", "jpg", "webp", "txt", "svg"],
            Family::Archive => &["zip", "7z", "tar.gz", "tar.xz", "tar"],
            Family::Subtitle => &["srt", "vtt", "ass", "txt", "ttml"],
        },
    };

    let targets = targets_for(source, tools);
    let mut out = Vec::new();
    for candidate in ranked {
        if *candidate == source.id {
            continue;
        }
        if targets.iter().any(|target| target == candidate)
            && !out.iter().any(|item| item == candidate)
        {
            out.push((*candidate).to_string());
            if out.len() == 5 {
                break;
            }
        }
    }
    if out.is_empty() {
        if let Some(first) = targets.iter().find(|target| target.as_str() != source.id) {
            out.push(first.clone());
        }
    }
    out
}

pub fn catalog(tools: &Tools) -> Catalog {
    let mut formats = Vec::new();
    let mut suggestion_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut available_count = 0;

    for spec in specs() {
        let a = available(spec, tools);
        if !a.read && !a.write {
            continue;
        }
        available_count += 1;
        let targets = targets_for(spec, tools);
        if a.read {
            let ranked = suggestions(spec, tools);
            if !ranked.is_empty() {
                suggestion_map.insert(spec.id.to_string(), ranked);
            }
        }
        formats.push(FormatInfo {
            id: spec.id.into(),
            label: spec.label.into(),
            family: spec.family.id().into(),
            extensions: spec.extensions.iter().map(|e| e.to_string()).collect(),
            mime: spec.mime.into(),
            note: if spec.note.is_empty() { None } else { Some(spec.note.into()) },
            lossless: spec.lossless,
            can_read: a.read,
            can_write: a.write,
            targets,
        });
    }

    // Stable ordering: family order then registry order.
    let order = |f: &str| match f {
        "image" => 0,
        "vector" => 1,
        "video" => 2,
        "audio" => 3,
        "document" => 4,
        "spreadsheet" => 5,
        "presentation" => 6,
        "pdf" => 7,
        "subtitle" => 8,
        "archive" => 9,
        _ => 10,
    };
    formats.sort_by_key(|f| (order(&f.family), f.label.clone()));

    let mut cross = Vec::new();
    if tools.has_video() {
        cross.push("video-to-audio".into());
        cross.push("video-to-frame".into());
        cross.push("video-to-gif".into());
        cross.push("audio-to-video".into());
    }
    if tools.has_images() {
        cross.push("image-to-video".into());
        cross.push("image-to-pdf".into());
    }
    if tools.has_pdf_read() && tools.has_images() {
        cross.push("pdf-to-image".into());
    }
    if tools.has_documents() {
        cross.push("document-to-pdf".into());
    }
    if tools.has_pdf_text() {
        cross.push("pdf-to-text".into());
    }
    if tools.has_audio_encoder("mp3") {
        cross.push("audio-to-waveform".into());
    }

    Catalog {
        format_count: formats.len(),
        available_count,
        formats,
        tools: tools.as_tool_infos(),
        suggestions: suggestion_map,
        cross,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

pub fn mode_label(source: Family, target: &Spec) -> String {
    match (source, target.family) {
        (Family::Video, Family::Image) if target.id == "gif" => "Make GIF".into(),
        (Family::Video, Family::Image) => "Extract frame".into(),
        (Family::Video, Family::Audio) => "Extract audio".into(),
        (Family::Audio, Family::Video) => "Visualise".into(),
        (Family::Audio, Family::Image) => "Waveform".into(),
        (Family::Image, Family::Video) => "Animate".into(),
        (Family::Image, Family::Pdf) => "Compose PDF".into(),
        (Family::Pdf, Family::Image) => "Rasterise".into(),
        (Family::Pdf, Family::Vector) => "Convert vector".into(),
        (Family::Pdf, Family::Document) => "Extract text".into(),
        (Family::Subtitle, Family::Document) => "Flatten".into(),
        (_, Family::Pdf) => "Export PDF".into(),
        (Family::Document, Family::Image) => "Render page".into(),
        (Family::Spreadsheet, Family::Image) => "Render sheet".into(),
        (Family::Presentation, Family::Image) => "Render slide".into(),
        _ => "Convert".into(),
    }
}

pub fn format_label(id: &str) -> String {
    lookup(id).map(|s| s.label.to_string()).unwrap_or_else(|| id.to_uppercase())
}

#[allow(dead_code)]
pub fn readable_ids(tools: &Tools) -> Vec<String> {
    let mut v = Vec::new();
    for family in [
        Family::Image,
        Family::Vector,
        Family::Video,
        Family::Audio,
        Family::Document,
        Family::Spreadsheet,
        Family::Presentation,
        Family::Pdf,
        Family::Archive,
        Family::Subtitle,
    ] {
        v.extend(ids(&readables(tools, family)));
    }
    v
}
