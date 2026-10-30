export type Family =
  | "image"
  | "vector"
  | "video"
  | "audio"
  | "document"
  | "spreadsheet"
  | "presentation"
  | "pdf"
  | "archive"
  | "subtitle";

export type ThemeId =
  | "system"
  | "dark"
  | "ink"
  | "midnight"
  | "ember"
  | "light"
  | "paper"
  | "linen";

export type ThemeMode = "dark" | "light";

export interface FormatInfo {
  id: string;
  label: string;
  family: Family;
  extensions: string[];
  mime: string;
  note: string | null;
  lossless: boolean;
  canRead: boolean;
  canWrite: boolean;
  targets: string[];
}

export interface ToolInfo {
  id: string;
  name: string;
  role: string;
  available: boolean;
  path: string | null;
  version: string | null;
  note: string | null;
}

export interface Catalog {
  formats: FormatInfo[];
  tools: ToolInfo[];
  suggestions: Record<string, string[]>;
  formatCount: number;
  availableCount: number;
  cross: string[];
  appVersion: string;
}

export interface FileMeta {
  path: string;
  name: string;
  extension: string;
  formatId: string;
  formatLabel: string;
  family: Family | "other";
  mime: string;
  size: number;
  modified: number;
  width: number | null;
  height: number | null;
  duration: number | null;
  fps: number | null;
  bitrate: number | null;
  sampleRate: number | null;
  channels: number | null;
  pages: number | null;
  videoCodec: string | null;
  audioCodec: string | null;
  hasVideo: boolean;
  hasAudio: boolean;
  previewKind: string;
  detail: string | null;
  thumbnail: string | null;
  error: string | null;
}

export interface FilmFrame {
  time: number;
  image: string;
}

export interface Options {
  quality?: number | null;
  maxDimension?: number | null;
  videoHeight?: number | null;
  fps?: number | null;
  trimStart?: number | null;
  trimEnd?: number | null;
  frameTime?: number | null;
  audioBitrate?: number | null;
  sampleRate?: number | null;
  channels?: number | null;
  normalizeAudio?: boolean | null;
  mute?: boolean | null;
  stripMetadata?: boolean | null;
  page?: number | null;
  dpi?: number | null;
  compression?: number | null;
  stillDuration?: number | null;
  keepSubtitles?: boolean | null;
}

export interface ConvertRequest {
  inputs: string[];
  target: string;
  options: Options;
  outputDir?: string | null;
}

export interface StartResponse {
  jobs: JobView[];
}

export type JobStatus = "queued" | "running" | "done" | "error" | "cancelled";

export interface JobView {
  id: string;
  inputs: string[];
  inputName: string;
  inputCount: number;
  output: string;
  outputName: string;
  target: string;
  targetLabel: string;
  targetFamily: Family;
  mode: string;
  status: JobStatus;
  progress: number;
  stage: string;
  error: string | null;
  sizeBefore: number;
  sizeAfter: number | null;
  createdAt: string;
  finishedAt: string | null;
}

export interface Settings {
  outputMode: "source" | "custom";
  outputDir: string;
  filenameTemplate: string;
  overwrite: "rename" | "overwrite" | "skip";
  concurrency: number;
  theme: ThemeId;
  defaultQuality: number;
  keepMetadata: boolean;
  reduceMotion: boolean;
  revealOnFinish: boolean;
  defaultTargets: Record<string, string>;
}

export type ViewId = "convert" | "formats" | "settings";

export interface Toast {
  id: string;
  title: string;
  description?: string;
  tone?: "neutral" | "success" | "error";
  actions?: { label: string; onClick: () => void }[];
}

export function emptyOptions(): Options {
  return {};
}
