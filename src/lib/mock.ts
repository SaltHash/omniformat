import catalogJson from "./mockCatalog.json";
import type {
  Catalog,
  ConvertRequest,
  FileMeta,
  FilmFrame,
  FormatInfo,
  JobView,
  Settings,
} from "./types";

const catalog = catalogJson as unknown as Catalog;

const defaultSettings: Settings = {
  outputMode: "source",
  outputDir: "",
  filenameTemplate: "{name}",
  overwrite: "rename",
  concurrency: 2,
  theme: "dark",
  defaultQuality: 85,
  keepMetadata: true,
  reduceMotion: false,
  revealOnFinish: true,
  defaultTargets: {},
};

let settings: Settings = { ...defaultSettings };
try {
  const saved = localStorage.getItem("convert.demo.settings");
  if (saved) settings = { ...settings, ...JSON.parse(saved) };
} catch {
  /* ignore */
}

const pasted = new Map<string, string>();
const listeners = new Set<(job: JobView) => void>();
const timers = new Map<string, number>();
let jobs: JobView[] = [];

export function mockCatalog(): Catalog {
  return catalog;
}

function basename(path: string): string {
  const cleaned = path.replace(/\\/g, "/");
  return cleaned.slice(cleaned.lastIndexOf("/") + 1) || cleaned;
}

function extensionOf(name: string): string {
  const lower = name.toLowerCase();
  const multi = catalog.formats
    .flatMap((f) => f.extensions.map((e) => ({ e, id: f.id })))
    .sort((a, b) => b.e.length - a.e.length)
    .find((item) => lower.endsWith(`.${item.e}`));
  return multi?.e ?? lower.split(".").pop() ?? "";
}

function formatForName(name: string): FormatInfo | null {
  const ext = extensionOf(name);
  return catalog.formats.find((f) => f.extensions.includes(ext)) ?? null;
}

function hash(text: string): number {
  let value = 0;
  for (let i = 0; i < text.length; i += 1) {
    value = (value * 31 + text.charCodeAt(i)) >>> 0;
  }
  return value;
}

export function mockInspect(path: string): FileMeta {
  const name = basename(path);
  const format = formatForName(name);
  const fallbackId = extensionOf(name) || "unknown";
  const family = format?.family ?? "other";
  const seed = hash(name);
  const image = family === "image" || family === "vector" || family === "pdf";
  const video = family === "video";
  const audio = family === "audio";
  const isPasted = pasted.get(path);
  const thumb = isPasted ?? (image ? "/demo-preview.jpg" : video ? "/demo-film-01.jpg" : audio ? "/demo-wave.png" : null);

  return {
    path,
    name,
    extension: extensionOf(name),
    formatId: format?.id ?? fallbackId,
    formatLabel: format?.label ?? fallbackId.toUpperCase(),
    family,
    mime: format?.mime ?? "application/octet-stream",
    size: 700_000 + (seed % 4_300_000),
    modified: Date.now(),
    width: image || video ? 1400 : null,
    height: image || video ? 900 : null,
    duration: video ? 18.4 : audio ? 192 : null,
    fps: video ? 24 : null,
    bitrate: video ? 8_400_000 : audio ? 320_000 : null,
    sampleRate: audio ? 48000 : null,
    channels: audio ? 2 : null,
    pages: family === "pdf" ? 6 : null,
    videoCodec: video ? "h264" : null,
    audioCodec: video ? "aac" : audio ? "flac" : null,
    hasVideo: video,
    hasAudio: video || audio,
    previewKind: video ? "video" : audio ? "audio" : image ? "image" : "document",
    detail: null,
    thumbnail: thumb,
    error: format ? null : "This type is not recognised",
  };
}

export function mockInspectPaths(paths: string[]): FileMeta[] {
  return paths.map(mockInspect);
}

export function mockFilmstrip(path: string): FilmFrame[] {
  const meta = mockInspect(path);
  if (meta.family !== "video") return [];
  return Array.from({ length: 8 }, (_, index) => ({
    time: Number(((index * 18.4) / 8).toFixed(2)),
    image: `/demo-film-0${index + 1}.jpg`,
  }));
}

export function mockFrameAt(_path: string, _time: number): string {
  return "/demo-preview.jpg";
}

export function mockClipboardFiles(): string[] {
  return [];
}

let pasteCount = 0;
export function mockSaveImage(data: string, mime: string): string {
  pasteCount += 1;
  const path = `/home/you/Pasted/pasted-image-${pasteCount}.${mime.split("/")[1] ?? "png"}`;
  pasted.set(path, `data:${mime};base64,${data}`);
  return path;
}

export function mockGetSettings(): Settings {
  return { ...settings };
}

export function mockSetSettings(next: Settings): Settings {
  settings = { ...next };
  try {
    localStorage.setItem("convert.demo.settings", JSON.stringify(settings));
  } catch {
    /* ignore */
  }
  return { ...settings };
}

function emit(job: JobView) {
  listeners.forEach((listener) => listener({ ...job }));
}

function makeJobView(inputs: string[], target: FormatInfo, request: ConvertRequest, id: string): JobView {
  const first = basename(inputs[0]);
  const stem = first.replace(/\.[^.]+$/, "");
  const ext = target.extensions[0] ?? target.id;
  const merged = inputs.length > 1;
  const outputName = `${stem}${merged ? "-combined" : ""}.${ext}`;
  return {
    id,
    inputs,
    inputName: merged ? `${inputs.length} files` : first,
    inputCount: inputs.length,
    output: `/home/you/Converted/${outputName}`,
    outputName,
    target: target.id,
    targetLabel: target.label,
    targetFamily: target.family,
    mode: "Convert",
    status: "queued",
    progress: 0,
    stage: "Queued",
    error: null,
    sizeBefore: inputs.reduce((sum, input) => sum + mockInspect(input).size, 0),
    sizeAfter: null,
    createdAt: new Date().toISOString(),
    finishedAt: null,
  };
}

export function mockStartConversion(request: ConvertRequest): JobView[] {
  const target = catalog.formats.find((f) => f.id === request.target);
  if (!target) throw new Error("Unknown target format");
  const inputs = request.inputs.filter(Boolean);
  const allImages = inputs.every((path) => mockInspect(path).family === "image");
  const merged = inputs.length > 1 && target.id === "pdf" && allImages;
  const groups = merged ? [inputs] : inputs.map((input) => [input]);
  const created = groups.map((group, index) =>
    makeJobView(group, target, request, `demo-job-${Date.now()}-${index}`),
  );

  created.forEach((job) => {
    jobs = [job, ...jobs.filter((item) => item.id !== job.id)];
    emit(job);
    const started = Date.now();
    const total = 2400 + (hash(job.inputName) % 2600);
    const timer = window.setInterval(() => {
      const elapsed = Date.now() - started;
      const progress = Math.min(99, Math.round((elapsed / total) * 100));
      const current = jobs.find((item) => item.id === job.id);
      if (!current || current.status === "cancelled") return;
      current.status = "running";
      current.progress = progress;
      current.stage =
        progress < 18
          ? "Preparing"
          : progress < 82
            ? target.family === "video"
              ? "Encoding"
              : target.family === "audio"
                ? "Encoding audio"
                : target.family === "archive"
                  ? "Packing"
                  : "Rendering"
            : "Finalising";
      emit(current);
      if (progress >= 99) {
        window.clearInterval(timer);
        timers.delete(job.id);
        current.status = "done";
        current.progress = 100;
        current.stage = "Done";
        current.sizeAfter = Math.round(current.sizeBefore * 0.64);
        current.finishedAt = new Date().toISOString();
        emit(current);
      }
    }, 180);
    timers.set(job.id, timer);
  });

  return created;
}

export function mockCancelJob(id: string): void {
  const timer = timers.get(id);
  if (timer) window.clearInterval(timer);
  timers.delete(id);
  const job = jobs.find((item) => item.id === id);
  if (job) {
    job.status = "cancelled";
    job.stage = "Cancelled";
    job.finishedAt = new Date().toISOString();
    emit(job);
  }
}

export function mockListJobs(): JobView[] {
  return [...jobs];
}

export function mockClearJobs(): void {
  jobs = jobs.filter((job) => job.status === "queued" || job.status === "running");
}

export function mockSubscribe(listener: (job: JobView) => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function mockDemoPath(kind: string | null): string {
  switch (kind) {
    case "video":
      return "/home/you/Videos/motion-study.mp4";
    case "audio":
      return "/home/you/Music/field-recording.flac";
    case "pdf":
      return "/home/you/Documents/quarterly-report.pdf";
    case "archive":
      return "/home/you/Downloads/archive.zip";
    case "document":
      return "/home/you/Documents/launch-notes.docx";
    default:
      return "/home/you/Pictures/still-life.png";
  }
}
