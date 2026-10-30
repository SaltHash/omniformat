import type { FileMeta, FormatInfo, Options, Settings } from "./types";

export function defaultOptionsFor(
  file: FileMeta | null,
  target: FormatInfo | null,
  settings: Settings | null,
): Options {
  if (!file || !target) return {};
  const quality = settings?.defaultQuality ?? 85;
  const keepMetadata = settings?.keepMetadata ?? true;
  const base: Options = {
    quality,
    stripMetadata: !keepMetadata,
    trimStart: null,
    trimEnd: null,
    mute: false,
    normalizeAudio: false,
  };

  const source = file.family;
  const family = target.family;

  if (family === "image" || family === "vector") {
    base.maxDimension = 0;
    if (target.lossless) base.quality = undefined;
    if (source === "video") {
      base.frameTime =
        file.duration && file.duration > 1
          ? Math.min(1, Math.max(0.1, file.duration * 0.1))
          : 0;
    }
    if (source === "pdf") {
      base.page = 1;
      base.dpi = 150;
    }
    return base;
  }

  if (family === "video") {
    base.videoHeight = 0;
    base.fps = 0;
    base.mute = false;
    if (source === "image" || source === "vector") {
      base.stillDuration = 5;
    }
    return base;
  }

  if (family === "audio") {
    base.audioBitrate = 192;
    base.sampleRate = 0;
    base.channels = 0;
    return base;
  }

  if (family === "pdf") {
    base.quality = 90;
    return base;
  }

  if (family === "archive") {
    base.compression = 5;
    return base;
  }

  return base;
}

export function normalizeOptions(options: Options): Options {
  const clean: Options = { ...options };
  const numericKeys: (keyof Options)[] = [
    "maxDimension",
    "videoHeight",
    "fps",
    "sampleRate",
    "channels",
    "audioBitrate",
    "page",
    "dpi",
    "compression",
    "stillDuration",
    "quality",
  ];
  numericKeys.forEach((key) => {
    const value = clean[key];
    if (value === 0 || value === null || value === undefined) {
      delete clean[key];
    }
  });
  (["trimStart", "trimEnd"] as (keyof Options)[]).forEach((key) => {
    if (clean[key] === null || clean[key] === undefined || Number.isNaN(clean[key])) {
      delete clean[key];
    }
  });
  return clean;
}

export function qualityRelevant(target: FormatInfo | null, file: FamilyLike | null): boolean {
  if (!target) return false;
  if (target.family === "image" || target.family === "vector") {
    if (file === "audio" || file === "video") return true;
    return !target.lossless || target.id === "png" || target.id === "tiff";
  }
  return target.family === "video" || target.family === "pdf";
}

type FamilyLike = string | null;
