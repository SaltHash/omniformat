import {
  Archive,
  Captions,
  FileText,
  Film,
  Image as ImageIcon,
  Music2,
  PenTool,
  Presentation,
  Table2,
  FileType2,
  type LucideIcon,
} from "lucide-react";
import type { Catalog, Family, FileMeta, FormatInfo } from "./types";

export interface FamilyMeta {
  id: Family;
  label: string;
  plural: string;
  blurb: string;
  color: string;
  icon: LucideIcon;
}

export const FAMILIES: Record<Family, FamilyMeta> = {
  image: {
    id: "image",
    label: "Image",
    plural: "Images",
    blurb: "Raster frames, layered files and camera raw.",
    color: "#e8a86b",
    icon: ImageIcon,
  },
  vector: {
    id: "vector",
    label: "Vector",
    plural: "Vector",
    blurb: "Resolution-independent artwork and PostScript.",
    color: "#b6a6e3",
    icon: PenTool,
  },
  video: {
    id: "video",
    label: "Video",
    plural: "Video",
    blurb: "Containers, codecs and motion.",
    color: "#d98e7a",
    icon: Film,
  },
  audio: {
    id: "audio",
    label: "Audio",
    plural: "Audio",
    blurb: "Lossless masters and portable mixes.",
    color: "#8fbfa3",
    icon: Music2,
  },
  document: {
    id: "document",
    label: "Document",
    plural: "Documents",
    blurb: "Text, office files and ebooks.",
    color: "#9bb2c9",
    icon: FileText,
  },
  spreadsheet: {
    id: "spreadsheet",
    label: "Spreadsheet",
    plural: "Spreadsheets",
    blurb: "Sheets, tables and delimited data.",
    color: "#a8c09b",
    icon: Table2,
  },
  presentation: {
    id: "presentation",
    label: "Presentation",
    plural: "Presentations",
    blurb: "Decks and slide exports.",
    color: "#c9a6a6",
    icon: Presentation,
  },
  pdf: {
    id: "pdf",
    label: "PDF",
    plural: "PDF",
    blurb: "Portable pages, print and text layers.",
    color: "#c9b99b",
    icon: FileType2,
  },
  archive: {
    id: "archive",
    label: "Archive",
    plural: "Archives",
    blurb: "Containers, compression and packages.",
    color: "#aaaaaa",
    icon: Archive,
  },
  subtitle: {
    id: "subtitle",
    label: "Subtitle",
    plural: "Subtitles",
    blurb: "Timed text for any player.",
    color: "#b0a8c9",
    icon: Captions,
  },
};

export const FAMILY_ORDER: Family[] = [
  "image",
  "vector",
  "video",
  "audio",
  "document",
  "spreadsheet",
  "presentation",
  "pdf",
  "archive",
  "subtitle",
];

export function familyMeta(family: string | null | undefined): FamilyMeta {
  if (family && family in FAMILIES) {
    return FAMILIES[family as Family];
  }
  return {
    id: "document",
    label: "File",
    plural: "Files",
    blurb: "An unrecognised file.",
    color: "#a8a39a",
    icon: FileText,
  };
}

export function formatById(catalog: Catalog | null, id: string | null | undefined): FormatInfo | null {
  if (!catalog || !id) return null;
  return catalog.formats.find((f) => f.id === id) ?? null;
}

export function abbrev(format: FormatInfo | null, fallback = "FILE"): string {
  if (!format) return fallback;
  const id = format.id.toUpperCase();
  const known: Record<string, string> = {
    JPEG: "JPG",
    JPG: "JPG",
    TIFF: "TIF",
    AIFF: "AIF",
    MPEG: "MPG",
    M2TS: "MTS",
    APNG: "APNG",
    SVGZ: "SVGZ",
    TTML: "TTML",
    DOCX: "DOCX",
    PPTX: "PPTX",
    XLSX: "XLSX",
    TAR: "TAR",
    "TAR.GZ": "TGZ",
    "TAR.BZ2": "TBZ",
    "TAR.XZ": "TXZ",
    "TAR.ZST": "TZS",
    EPUB: "EPUB",
  };
  return known[id] ?? (id.length > 4 ? id.slice(0, 4) : id);
}

export function extensionList(format: FormatInfo | null): string {
  if (!format) return "";
  return format.extensions.map((e) => `.${e}`).join(" ");
}

export function shortPath(path: string | null | undefined): string {
  if (!path) return "";
  return path.replace(/^\/home\/[^/]+\//, "~/");
}

export function humanSize(bytes: number | null | undefined): string {
  if (bytes == null || !Number.isFinite(bytes)) return "—";
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value >= 100 ? value.toFixed(0) : value >= 10 ? value.toFixed(1) : value.toFixed(2)} ${units[unit]}`;
}

export function durationLabel(seconds: number | null | undefined): string {
  if (seconds == null || !Number.isFinite(seconds)) return "—";
  const total = Math.max(0, Math.round(seconds));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  if (h > 0) return `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
  return `${m}:${String(s).padStart(2, "0")}`;
}

export function timecode(seconds: number | null | undefined): string {
  if (seconds == null || !Number.isFinite(seconds)) return "0:00.0";
  const s = Math.max(0, seconds);
  const whole = Math.floor(s);
  const tenths = Math.floor((s - whole) * 10);
  const m = Math.floor(whole / 60);
  const sec = whole % 60;
  return `${m}:${String(sec).padStart(2, "0")}.${tenths}`;
}

export function fileDetail(file: FileMeta | null): string {
  if (!file) return "";
  const parts: string[] = [file.formatLabel];
  if (file.width && file.height) parts.push(`${file.width} × ${file.height}`);
  if (file.family === "video" && file.duration) parts.push(durationLabel(file.duration));
  if (file.family === "audio" && file.duration) parts.push(durationLabel(file.duration));
  if (file.pages && file.pages > 1) parts.push(`${file.pages} pages`);
  parts.push(humanSize(file.size));
  return parts.join(" · ");
}

export function modeLabel(source: string | null | undefined, target: FormatInfo | null): string {
  if (!source || !target) return "Convert";
  if (source === "video" && target.family === "audio") return "Extract audio";
  if (source === "video" && target.family === "image") {
    return target.id === "gif" ? "Make GIF" : "Extract frame";
  }
  if (source === "audio" && target.family === "video") return "Visualise";
  if (source === "audio" && target.family === "image") return "Waveform";
  if (source === "image" && target.family === "video") return "Animate";
  if (source === "image" && target.family === "pdf") return "Compose PDF";
  if (source === "pdf" && target.family === "image") return "Rasterise";
  if (source === "pdf" && target.family === "vector") return "Vectorise";
  if (source === "pdf" && target.family === "document") return "Extract text";
  if (source === "subtitle" && target.family === "document") return "Flatten";
  if (source === target.family) {
    if (source === "image" || source === "vector") return "Re-encode";
    if (source === "video" || source === "audio") return "Transcode";
    if (source === "archive") return "Repack";
  }
  if (target.family === "pdf") return "Export PDF";
  if (source === "document" && target.family === "image") return "Render page";
  if (source === "spreadsheet" && target.family === "image") return "Render sheet";
  if (source === "presentation" && target.family === "image") return "Render slide";
  return "Convert";
}

export function targetTag(source: string | null | undefined, target: FormatInfo): string | null {
  if (source === "video" && target.family === "audio") return "Audio";
  if (source === "video" && target.family === "image") return target.id === "gif" ? "GIF" : "Frame";
  if (source === "audio" && target.family === "video") return "Visual";
  if (source === "audio" && target.family === "image") return "Wave";
  if (source === "image" && target.family === "video") return "Motion";
  if (source === "image" && target.family === "pdf") return "PDF";
  if (source === "pdf" && target.family !== "pdf") return modeLabel(source, target);
  if (source === "subtitle" && target.family === "document") return "Text";
  return null;
}

export function groupFormatsByFamily(formats: FormatInfo[]): { family: Family; items: FormatInfo[] }[] {
  return FAMILY_ORDER.map((family) => ({
    family,
    items: formats
      .filter((item) => item.family === family)
      .sort((a, b) => {
        const readDiff = Number(b.canWrite) - Number(a.canWrite);
        if (readDiff !== 0) return readDiff;
        return a.label.localeCompare(b.label);
      }),
  })).filter((group) => group.items.length > 0);
}
