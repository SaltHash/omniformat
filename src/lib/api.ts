import type {
  Catalog,
  ConvertRequest,
  FileMeta,
  FilmFrame,
  JobView,
  Settings,
  StartResponse,
} from "./types";

export const isTauri =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

let mockModule: Promise<typeof import("./mock")> | null = null;
function getMock(): Promise<typeof import("./mock")> {
  if (!mockModule) mockModule = import("./mock");
  return mockModule;
}

export function errorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  if (error && typeof error === "object" && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return "Something went wrong";
}

export async function getCatalog(): Promise<Catalog> {
  if (!isTauri) return (await getMock()).mockCatalog();
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<Catalog>("get_catalog");
}

export async function refreshCatalog(): Promise<Catalog> {
  if (!isTauri) return (await getMock()).mockCatalog();
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<Catalog>("refresh_catalog");
}

export async function inspectPaths(paths: string[]): Promise<FileMeta[]> {
  if (!isTauri) return (await getMock()).mockInspectPaths(paths);
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<FileMeta[]>("inspect_paths", { paths });
}

export async function getFilmstrip(path: string): Promise<FilmFrame[]> {
  if (!isTauri) return (await getMock()).mockFilmstrip(path);
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<FilmFrame[]>("filmstrip", { path });
}

export async function getFrame(path: string, time: number): Promise<string> {
  if (!isTauri) return (await getMock()).mockFrameAt(path, time);
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<string>("frame_at", { path, time });
}

export async function clipboardFiles(): Promise<string[]> {
  if (!isTauri) return (await getMock()).mockClipboardFiles();
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<string[]>("clipboard_files");
}

export async function saveTempImage(data: string, mime: string): Promise<string> {
  if (!isTauri) return (await getMock()).mockSaveImage(data, mime);
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<string>("save_temp_image", { data, mime });
}

export async function startConversion(request: ConvertRequest): Promise<StartResponse> {
  if (!isTauri) {
    const jobs = (await getMock()).mockStartConversion(request);
    return { jobs };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<StartResponse>("start_conversion", { request });
}

export async function listJobs(): Promise<JobView[]> {
  if (!isTauri) return (await getMock()).mockListJobs();
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<JobView[]>("list_jobs");
}

export async function cancelJob(id: string): Promise<void> {
  if (!isTauri) {
    (await getMock()).mockCancelJob(id);
    return;
  }
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("cancel_job", { id });
}

export async function clearJobs(): Promise<void> {
  if (!isTauri) {
    (await getMock()).mockClearJobs();
    return;
  }
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("clear_jobs");
}

export async function getSettings(): Promise<Settings> {
  if (!isTauri) return (await getMock()).mockGetSettings();
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<Settings>("get_settings");
}

export async function setSettings(settings: Settings): Promise<Settings> {
  if (!isTauri) return (await getMock()).mockSetSettings(settings);
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<Settings>("set_settings", { settings });
}

export async function clearCache(): Promise<void> {
  if (!isTauri) return;
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("clear_cache");
}

export async function openOutput(path: string, reveal: boolean): Promise<void> {
  if (!isTauri) return;
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("open_output", { path, reveal });
}

export async function configLocation(): Promise<string> {
  if (!isTauri) return "~/.config/convert";
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<string>("config_location");
}

export async function pickFiles(): Promise<string[]> {
  if (isTauri) {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const result = await open({
      multiple: true,
      directory: false,
      title: "Choose files",
    });
    if (!result) return [];
    return Array.isArray(result) ? result : [result];
  }
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.multiple = true;
    input.style.display = "none";
    document.body.appendChild(input);
    input.onchange = () => {
      const names = Array.from(input.files ?? []).map(
        (file) => `/home/you/Downloads/${file.name}`,
      );
      input.remove();
      resolve(names);
    };
    input.oncancel = () => {
      input.remove();
      resolve([]);
    };
    input.click();
  });
}

export async function pickFolder(): Promise<string | null> {
  if (isTauri) {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const result = await open({
      directory: true,
      multiple: false,
      title: "Choose an output folder",
    });
    return typeof result === "string" ? result : null;
  }
  return null;
}

export async function subscribeJobs(
  callback: (job: JobView) => void,
): Promise<() => void> {
  if (!isTauri) {
    return (await getMock()).mockSubscribe(callback);
  }
  const { listen } = await import("@tauri-apps/api/event");
  const unlisten = await listen<JobView>("job:update", (event) =>
    callback(event.payload),
  );
  return unlisten;
}
