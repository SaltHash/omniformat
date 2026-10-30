import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import * as api from "./lib/api";
import { defaultOptionsFor, normalizeOptions } from "./lib/options";
import { formatById, shortPath } from "./lib/families";
import type {
  Catalog,
  Family,
  FileMeta,
  FormatInfo,
  JobView,
  Options,
  Settings,
  ThemeId,
  ThemeMode,
  ViewId,
} from "./lib/types";
import { ActivityTray } from "./components/ActivityTray";
import { CommandPalette } from "./components/CommandPalette";
import { Rail } from "./components/Rail";
import { TitleBar } from "./components/TitleBar";
import { ToastProvider, useToast } from "./components/Toasts";
import { Mark, Spinner } from "./components/ui";
import { ConvertPage } from "./pages/ConvertPage";
import { FormatsPage } from "./pages/FormatsPage";
import { SettingsPage } from "./pages/SettingsPage";

const VIEWS: ViewId[] = ["convert", "formats", "settings"];

function readView(): ViewId {
  const hash = window.location.hash.replace(/^#\/?/, "");
  return (VIEWS.includes(hash as ViewId) ? hash : "convert") as ViewId;
}

function mergeJob(list: JobView[], job: JobView): JobView[] {
  const index = list.findIndex((item) => item.id === job.id);
  if (index === -1) return [job, ...list];
  const next = [...list];
  next[index] = job;
  return next;
}

const DEMO_FILES: Record<string, string> = {
  image: "/home/you/Pictures/still-life.jpg",
  video: "/home/you/Videos/motion-study.mp4",
  audio: "/home/you/Music/late-set.wav",
  document: "/home/you/Documents/annual-notes.docx",
  pdf: "/home/you/Documents/edition-04.pdf",
  archive: "/home/you/Downloads/studio-share.zip",
};

interface FileStage {
  targetId: string | null;
  options: Options;
}

function targetsForFile(
  catalog: Catalog | null,
  file: FileMeta | null,
): FormatInfo[] {
  const info = file ? formatById(catalog, file.formatId) : null;
  if (!catalog || !info) return [];
  return info.targets
    .map((id) => formatById(catalog, id))
    .filter((format): format is FormatInfo => Boolean(format?.canWrite));
}

function resolveTargetId(
  file: FileMeta,
  catalog: Catalog | null,
  settings: Settings | null,
  preferred?: string | null,
): string | null {
  const info = formatById(catalog, file.formatId);
  if (!catalog || !info) return null;
  const writable = new Set(
    info.targets.filter((id) => formatById(catalog, id)?.canWrite),
  );
  const ranked = [
    preferred ?? undefined,
    settings?.defaultTargets?.[file.formatId],
    ...(catalog.suggestions[file.formatId] ?? []),
    ...info.targets,
  ];
  for (const id of ranked) {
    if (id && writable.has(id)) return id;
  }
  return null;
}

function stageForFile(
  file: FileMeta,
  catalog: Catalog | null,
  settings: Settings | null,
  preferred?: string | null,
): FileStage {
  const targetId = resolveTargetId(file, catalog, settings, preferred);
  return {
    targetId,
    options: defaultOptionsFor(
      file,
      formatById(catalog, targetId),
      settings,
    ),
  };
}

export function App() {
  return (
    <ToastProvider>
      <AppShell />
    </ToastProvider>
  );
}

function AppShell() {
  const { pushToast } = useToast();
  const [catalog, setCatalog] = useState<Catalog | null>(null);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [jobs, setJobs] = useState<JobView[]>([]);
  const [view, setView] = useState<ViewId>(readView);
  const [formatsFamily, setFormatsFamily] = useState<Family | null>(null);
  const [files, setFiles] = useState<FileMeta[]>([]);
  const [active, setActive] = useState(0);
  const [stage, setStage] = useState<Record<string, FileStage>>({});
  const [outputDir, setOutputDir] = useState<string | null>(null);
  const [inspecting, setInspecting] = useState(false);
  const [refreshing, setRefreshing] = useState(false);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [dragging, setDragging] = useState(false);
  const [systemDark, setSystemDark] = useState(
    () => window.matchMedia("(prefers-color-scheme: dark)").matches,
  );
  const [booting, setBooting] = useState(true);
  const [fatal, setFatal] = useState<string | null>(null);

  const filesRef = useRef<FileMeta[]>([]);
  const jobsRef = useRef<JobView[]>([]);
  const catalogRef = useRef<Catalog | null>(null);
  const settingsRef = useRef<Settings | null>(null);
  const stageRef = useRef<Record<string, FileStage>>({});
  const saveTimer = useRef<number | undefined>(undefined);
  const demoRan = useRef(false);

  filesRef.current = files;
  jobsRef.current = jobs;
  catalogRef.current = catalog;
  settingsRef.current = settings;
  stageRef.current = stage;

  const activeFile = files[active] ?? files[0] ?? null;
  const activeStage = activeFile ? stage[activeFile.path] : undefined;
  const targets = useMemo(
    () => targetsForFile(catalog, activeFile),
    [catalog, activeFile],
  );
  const target = useMemo(
    () => targets.find((format) => format.id === activeStage?.targetId) ?? null,
    [activeStage?.targetId, targets],
  );
  const options = activeStage?.options ?? ({} as Options);
  const perFileTargets = useMemo(() => {
    const map: Record<string, FormatInfo | null> = {};
    files.forEach((file) => {
      map[file.path] = formatById(catalog, stage[file.path]?.targetId);
    });
    return map;
  }, [catalog, files, stage]);
  const sharedOptions = useMemo(() => {
    if (files.length < 2) return true;
    const signatures = new Set(
      files.map((file) =>
        JSON.stringify(normalizeOptions(stage[file.path]?.options ?? {})),
      ),
    );
    return signatures.size === 1;
  }, [files, stage]);

  const resolvedTheme: ThemeId = settings
    ? settings.theme === "system"
      ? systemDark
        ? "dark"
        : "light"
      : settings.theme
    : "dark";
  const resolvedMode: ThemeMode =
    resolvedTheme === "light" || resolvedTheme === "paper" || resolvedTheme === "linen"
      ? "light"
      : "dark";

  /* ---------------------------------------------------------------- boot */

  const load = useCallback(async () => {
    setFatal(null);
    setBooting(true);
    try {
      const [cat, prefs, currentJobs] = await Promise.all([
        api.getCatalog(),
        api.getSettings(),
        api.listJobs(),
      ]);
      setCatalog(cat);
      setSettings(prefs);
      settingsRef.current = prefs;
      const sorted = [...currentJobs].sort((a, b) =>
        a.createdAt < b.createdAt ? 1 : -1,
      );
      setJobs(sorted);
      jobsRef.current = sorted;
    } catch (error) {
      setFatal(api.errorMessage(error));
    } finally {
      setBooting(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  /* --------------------------------------------------------- job stream */

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void api
      .subscribeJobs((job) => {
        const previous = jobsRef.current.find((item) => item.id === job.id);
        const next = mergeJob(jobsRef.current, job);
        jobsRef.current = next;
        setJobs(next);

        if (job.status === "done" && previous?.status !== "done") {
          if (settingsRef.current?.revealOnFinish) {
            void api.openOutput(job.output, true).catch(() => undefined);
          }
          pushToast({
            tone: "success",
            title: `${job.targetLabel} ready`,
            description: job.outputName,
            actions: [
              {
                label: "Show",
                onClick: () => {
                  void api
                    .openOutput(job.output, true)
                    .catch((error) =>
                      pushToast({
                        tone: "error",
                        title: "Could not open the folder",
                        description: api.errorMessage(error),
                      }),
                    );
                },
              },
            ],
          });
        }

        if (job.status === "error" && previous?.status !== "error") {
          pushToast({
            tone: "error",
            title: "Conversion failed",
            description: job.error ?? "The engine stopped unexpectedly.",
          });
        }
      })
      .then((stop) => {
        if (disposed) stop();
        else unlisten = stop;
      })
      .catch(() => undefined);
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [pushToast]);

  /* ------------------------------------------------------------ theme */

  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const update = () => setSystemDark(media.matches);
    media.addEventListener("change", update);
    return () => media.removeEventListener("change", update);
  }, []);

  useEffect(() => {
    document.documentElement.dataset.theme = resolvedTheme;
  }, [resolvedTheme]);

  useEffect(() => {
    document.documentElement.dataset.motion =
      settings?.reduceMotion ? "off" : "on";
  }, [settings?.reduceMotion]);

  /* ----------------------------------------------------------- routing */

  useEffect(() => {
    const onHash = () => setView(readView());
    window.addEventListener("hashchange", onHash);
    return () => window.removeEventListener("hashchange", onHash);
  }, []);

  const navigate = useCallback((next: ViewId) => {
    setView(next);
    setPaletteOpen(false);
    if (next === "formats") setFormatsFamily(null);
    window.history.replaceState(null, "", `#${next}`);
  }, []);

  const showFormats = useCallback((family?: Family) => {
    setFormatsFamily(family ?? null);
    setView("formats");
    setPaletteOpen(false);
    window.history.replaceState(null, "", "#formats");
  }, []);

  /* ----------------------------------------------------------- settings */

  const updateSettings = useCallback(
    (patch: Partial<Settings>) => {
      const current = settingsRef.current;
      if (!current) return;
      const next = { ...current, ...patch };
      settingsRef.current = next;
      setSettings(next);
      if (saveTimer.current) window.clearTimeout(saveTimer.current);
      saveTimer.current = window.setTimeout(() => {
        void api
          .setSettings(next)
          .then((saved) => {
            settingsRef.current = saved;
            setSettings(saved);
          })
          .catch((error) =>
            pushToast({
              tone: "error",
              title: "Could not save settings",
              description: api.errorMessage(error),
            }),
          );
      }, 280);
    },
    [pushToast],
  );

  useEffect(
    () => () => {
      if (saveTimer.current) window.clearTimeout(saveTimer.current);
    },
    [],
  );

  /* ------------------------------------------------------------- files */

  const addPaths = useCallback(
    async (
      paths: string[],
      addOptions?: { preferredTarget?: string; replace?: boolean },
    ) => {
      if (!paths.length) return;
      setInspecting(true);
      try {
        const inspected = await api.inspectPaths(paths);
        const good = inspected.filter(
          (file) => file.family !== "other" && !file.error,
        );
        const rejected = inspected.filter(
          (file) => file.family === "other" || file.error,
        );
        if (rejected.length) {
          pushToast({
            tone: "error",
            title:
              rejected.length === 1
                ? "That file cannot be converted"
                : `${rejected.length} files skipped`,
            description:
              rejected[0].error ??
              `${rejected[0].name} is not a recognised file type.`,
          });
        }
        if (!good.length) return;

        const current = filesRef.current;
        const base = addOptions?.replace ? [] : current;
        const map = new Map(base.map((file) => [file.path, file]));
        good.forEach((file) => map.set(file.path, file));
        const merged = Array.from(map.values());
        const firstNew = merged.findIndex(
          (file) => !base.some((item) => item.path === file.path),
        );
        const nextActive =
          firstNew >= 0
            ? firstNew
            : Math.min(base.length, merged.length - 1);
        const nextStage: Record<string, FileStage> = {};
        merged.forEach((file) => {
          const existing = addOptions?.replace
            ? undefined
            : stageRef.current[file.path];
          nextStage[file.path] =
            existing ??
            stageForFile(
              file,
              catalogRef.current,
              settingsRef.current,
              addOptions?.preferredTarget,
            );
        });
        setFiles(merged);
        filesRef.current = merged;
        setStage(nextStage);
        stageRef.current = nextStage;
        setActive(nextActive);
      } catch (error) {
        pushToast({
          tone: "error",
          title: "Could not read the file",
          description: api.errorMessage(error),
        });
      } finally {
        setInspecting(false);
      }
    },
    [pushToast],
  );

  // Keep every staged file pointed at a target the current toolchain can write.
  useEffect(() => {
    if (!catalog) return;
    setStage((prev) => {
      let changed = false;
      const next: Record<string, FileStage> = {};
      filesRef.current.forEach((file) => {
        const existing = prev[file.path];
        const allowed = formatById(catalog, file.formatId)?.targets ?? [];
        if (existing?.targetId && allowed.includes(existing.targetId)) {
          next[file.path] = existing;
        } else {
          next[file.path] = stageForFile(file, catalog, settingsRef.current);
          changed = true;
        }
      });
      if (
        !changed &&
        Object.keys(next).length === Object.keys(prev).length
      ) {
        return prev;
      }
      stageRef.current = next;
      return next;
    });
  }, [catalog, files]);

  const onSelectFile = useCallback((index: number) => {
    setActive(index);
  }, []);

  const onRemove = useCallback((index: number) => {
    const current = filesRef.current;
    const removed = current[index];
    const next = current.filter((_, itemIndex) => itemIndex !== index);
    setFiles(next);
    filesRef.current = next;
    if (removed) {
      setStage((prev) => {
        const copy = { ...prev };
        delete copy[removed.path];
        stageRef.current = copy;
        return copy;
      });
    }
    if (!next.length) {
      setActive(0);
      setOutputDir(null);
      return;
    }
    setActive(Math.min(active, next.length - 1));
  }, [active]);

  const onReplace = useCallback(async () => {
    const picked = await api.pickFiles();
    if (!picked.length) return;
    const inspected = await api.inspectPaths([picked[0]]);
    const file = inspected[0];
    if (!file || file.family === "other" || file.error) {
      pushToast({
        tone: "error",
        title: "That file cannot be converted",
        description: file?.error ?? "Try a different file.",
      });
      return;
    }
    const current = filesRef.current;
    if (!current.length) {
      await addPaths([file.path]);
      return;
    }
    const replaced = current[active];
    const previousTarget = replaced
      ? stageRef.current[replaced.path]?.targetId ?? null
      : null;
    const allowed = formatById(catalogRef.current, file.formatId)?.targets ?? [];
    const preferred =
      previousTarget && allowed.includes(previousTarget)
        ? previousTarget
        : null;
    const next = current.map((item, index) =>
      index === active ? file : item,
    );
    setFiles(next);
    filesRef.current = next;
    const nextStage = { ...stageRef.current };
    if (replaced) delete nextStage[replaced.path];
    nextStage[file.path] = stageForFile(
      file,
      catalogRef.current,
      settingsRef.current,
      preferred,
    );
    setStage(nextStage);
    stageRef.current = nextStage;
  }, [active, addPaths, pushToast]);

  const onClear = useCallback(() => {
    setFiles([]);
    filesRef.current = [];
    setActive(0);
    setStage({});
    stageRef.current = {};
    setOutputDir(null);
  }, []);

  const goHome = useCallback(() => {
    onClear();
    navigate("convert");
  }, [onClear, navigate]);

  const onTarget = useCallback(
    (format: FormatInfo) => {
      const file = filesRef.current[active] ?? filesRef.current[0] ?? null;
      if (!file) return;
      setStage((prev) => {
        const next = {
          ...prev,
          [file.path]: {
            targetId: format.id,
            options: defaultOptionsFor(file, format, settingsRef.current),
          },
        };
        stageRef.current = next;
        return next;
      });
    },
    [active],
  );

  const onOptions = useCallback(
    (patch: Partial<Options>) => {
      const file = filesRef.current[active] ?? filesRef.current[0] ?? null;
      if (!file) return;
      setStage((prev) => {
        const existing =
          prev[file.path] ??
          stageForFile(file, catalogRef.current, settingsRef.current);
        const next = {
          ...prev,
          [file.path]: {
            ...existing,
            options: { ...existing.options, ...patch },
          },
        };
        stageRef.current = next;
        return next;
      });
    },
    [active],
  );

  const applyTargetToAll = useCallback(() => {
    const file = filesRef.current[active] ?? filesRef.current[0] ?? null;
    const format = formatById(
      catalogRef.current,
      file ? stageRef.current[file.path]?.targetId : null,
    );
    if (!file || !format) return;
    const incompatible = filesRef.current.filter(
      (item) =>
        !(
          formatById(catalogRef.current, item.formatId)?.targets.includes(
            format.id,
          ) ?? false
        ),
    );
    if (incompatible.length) {
      pushToast({
        tone: "neutral",
        title: `${format.label} does not fit every file`,
        description: `${incompatible[0].name} has no ${format.label} output.`,
      });
      return;
    }
    const next: Record<string, FileStage> = {};
    filesRef.current.forEach((item) => {
      next[item.path] = {
        targetId: format.id,
        options: defaultOptionsFor(item, format, settingsRef.current),
      };
    });
    setStage(next);
    stageRef.current = next;
    pushToast({
      tone: "success",
      title: `${format.label} set for all ${filesRef.current.length} files`,
      description: "Adjust any file individually from its chip.",
    });
  }, [active, pushToast]);

  const browse = useCallback(async () => {
    try {
      const picked = await api.pickFiles();
      if (picked.length) await addPaths(picked);
    } catch (error) {
      pushToast({
        tone: "error",
        title: "Could not open the file picker",
        description: api.errorMessage(error),
      });
    }
  }, [addPaths, pushToast]);

  const paste = useCallback(async () => {
    try {
      const paths = await api.clipboardFiles();
      if (!paths.length) {
        pushToast({
          tone: "neutral",
          title: "Nothing on the clipboard",
          description:
            "Copy a file in your file manager, or take a screenshot, then try again.",
        });
        return;
      }
      await addPaths(paths);
    } catch (error) {
      pushToast({
        tone: "error",
        title: "Could not paste",
        description: api.errorMessage(error),
      });
    }
  }, [addPaths, pushToast]);

  /* ----------------------------------------------------------- convert */

  const converting = useMemo(() => {
    if (!files.length) return false;
    const paths = new Set(files.map((file) => file.path));
    return jobs.some(
      (job) =>
        (job.status === "queued" || job.status === "running") &&
        job.inputs.some((input) => paths.has(input)),
    );
  }, [files, jobs]);

  const convert = useCallback(async (folderOverride?: string) => {
    const currentFiles = filesRef.current;
    if (!currentFiles.length) return;
    const targetDir = folderOverride ?? outputDir;
    const catalogNow = catalogRef.current;
    const prepared = currentFiles.map((file) => {
      const entry = stageRef.current[file.path];
      const targetId =
        entry?.targetId ??
        resolveTargetId(file, catalogNow, settingsRef.current);
      const format = formatById(catalogNow, targetId);
      if (!targetId || !format) return null;
      return {
        file,
        targetId,
        options: normalizeOptions(
          entry?.options ??
            defaultOptionsFor(file, format, settingsRef.current),
        ),
      };
    });
    const ready = prepared.filter(
      (item): item is NonNullable<typeof item> => Boolean(item),
    );
    if (!ready.length) {
      pushToast({
        tone: "error",
        title: "No output format selected",
        description: "Choose a format for at least one file.",
      });
      return;
    }
    if (ready.length < currentFiles.length) {
      pushToast({
        tone: "neutral",
        title: `${currentFiles.length - ready.length} file${
          currentFiles.length - ready.length === 1 ? "" : "s"
        } skipped`,
        description: "No compatible output format was found for the rest.",
      });
    }
    const paths = new Set(ready.map((item) => item.file.path));
    const busy = jobsRef.current.some(
      (job) =>
        (job.status === "queued" || job.status === "running") &&
        job.inputs.some((input) => paths.has(input)),
    );
    if (busy) {
      pushToast({
        tone: "neutral",
        title: "Already converting",
        description: "These files are already in the activity queue.",
      });
      return;
    }

    // Files with the same format and adjustments travel together, which keeps
    // image-to-PDF and image-to-video batches merged. Everything else queues
    // as its own request.
    const groups = new Map<
      string,
      { target: string; options: Options; inputs: string[] }
    >();
    ready.forEach((item) => {
      const key = `${item.targetId}::${JSON.stringify(item.options)}`;
      const group = groups.get(key);
      if (group) group.inputs.push(item.file.path);
      else
        groups.set(key, {
          target: item.targetId,
          options: item.options,
          inputs: [item.file.path],
        });
    });

    const results = await Promise.allSettled(
      Array.from(groups.values()).map((group) =>
        api.startConversion({
          inputs: group.inputs,
          target: group.target,
          options: group.options,
          outputDir: targetDir,
        }),
      ),
    );
    let next = jobsRef.current;
    let failures = 0;
    let firstError = "";
    results.forEach((result) => {
      if (result.status === "fulfilled") {
        result.value.jobs.forEach((job) => {
          next = mergeJob(next, job);
        });
      } else {
        failures += 1;
        if (!firstError) firstError = api.errorMessage(result.reason);
      }
    });
    jobsRef.current = next;
    setJobs(next);
    if (failures) {
      pushToast({
        tone: "error",
        title:
          failures === 1
            ? "Could not start a conversion"
            : `${failures} batches could not start`,
        description: firstError || "The engine rejected the request.",
      });
    }
  }, [outputDir, pushToast]);

  const refreshCatalog = useCallback(async () => {
    setRefreshing(true);
    try {
      const next = await api.refreshCatalog();
      setCatalog(next);
      pushToast({
        tone: "success",
        title: "Toolchain rescanned",
        description: `${next.formatCount} formats across ${new Set(next.formats.map((format) => format.family)).size} families.`,
      });
    } catch (error) {
      pushToast({
        tone: "error",
        title: "Could not rescan tools",
        description: api.errorMessage(error),
      });
    } finally {
      setRefreshing(false);
    }
  }, [pushToast]);

  const revealOutput = useCallback(
    (path: string) => {
      void api.openOutput(path, true).catch((error) =>
        pushToast({
          tone: "error",
          title: "Could not open the folder",
          description: api.errorMessage(error),
        }),
      );
    },
    [pushToast],
  );

  const clearJobs = useCallback(async () => {
    setJobs((current) =>
      current.filter(
        (job) => job.status === "queued" || job.status === "running",
      ),
    );
    try {
      await api.clearJobs();
    } catch {
      /* tray stays usable either way */
    }
  }, []);

  const clearCache = useCallback(() => {
    void api
      .clearCache()
      .then(() =>
        pushToast({
          tone: "success",
          title: "Preview cache cleared",
          description: "Thumbnails regenerate when you select a file.",
        }),
      )
      .catch((error) =>
        pushToast({
          tone: "error",
          title: "Could not clear the cache",
          description: api.errorMessage(error),
        }),
      );
  }, [pushToast]);

  /* ------------------------------------------------------------ demo */

  useEffect(() => {
    if (api.isTauri || booting || !catalog || demoRan.current) return;
    const demo = new URLSearchParams(window.location.search).get("demo");
    if (!demo) return;
    demoRan.current = true;
    const path = DEMO_FILES[demo] ?? DEMO_FILES.image;
    const preferred = new URLSearchParams(window.location.search).get("target");
    void addPaths([path], {
      preferredTarget: preferred ?? undefined,
      replace: true,
    });
  }, [addPaths, booting, catalog]);

  /* --------------------------------------------------------- shortcuts */

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const mod = event.ctrlKey || event.metaKey;
      const element = event.target as HTMLElement | null;
      const tag = element?.tagName ?? "";
      const typing =
        tag === "INPUT" ||
        tag === "TEXTAREA" ||
        tag === "SELECT" ||
        messageIsEditable(element);

      if (mod && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setPaletteOpen((open) => !open);
        return;
      }
      if (mod && event.key.toLowerCase() === "o") {
        event.preventDefault();
        void browse();
        return;
      }
      if (
        mod &&
        event.key.toLowerCase() === "v" &&
        !typing &&
        api.isTauri &&
        !paletteOpen
      ) {
        event.preventDefault();
        void paste();
        return;
      }
      if (
        event.key === "Enter" &&
        !typing &&
        !mod &&
        !paletteOpen &&
        view === "convert" &&
        !event.shiftKey &&
        !event.altKey &&
        tag !== "BUTTON" &&
        tag !== "A"
      ) {
        event.preventDefault();
        void convert();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [browse, convert, paletteOpen, paste, view]);

  /* -------------------------------------------------------- drag & drop */

  useEffect(() => {
    if (api.isTauri) {
      let disposed = false;
      let unlisten: (() => void) | undefined;
      void (async () => {
        try {
          const { getCurrentWebview } = await import(
            "@tauri-apps/api/webview"
          );
          const stop = await getCurrentWebview().onDragDropEvent((event) => {
            if (event.payload.type === "enter") setDragging(true);
            if (event.payload.type === "over") setDragging(true);
            if (event.payload.type === "leave") setDragging(false);
            if (event.payload.type === "drop") {
              setDragging(false);
              void addPaths(event.payload.paths);
            }
          });
          if (disposed) stop();
          else unlisten = stop;
        } catch {
          /* drag events are an enhancement */
        }
      })();
      return () => {
        disposed = true;
        unlisten?.();
      };
    }

    let depth = 0;
    const isFileDrag = (event: DragEvent) =>
      Array.from(event.dataTransfer?.types ?? []).includes("Files");
    const onEnter = (event: DragEvent) => {
      if (!isFileDrag(event)) return;
      event.preventDefault();
      depth += 1;
      setDragging(true);
    };
    const onOver = (event: DragEvent) => {
      if (!isFileDrag(event)) return;
      event.preventDefault();
    };
    const onLeave = (event: DragEvent) => {
      if (!isFileDrag(event)) return;
      depth = Math.max(0, depth - 1);
      if (depth === 0) setDragging(false);
    };
    const onDrop = (event: DragEvent) => {
      if (!isFileDrag(event)) return;
      event.preventDefault();
      depth = 0;
      setDragging(false);
      const names = Array.from(event.dataTransfer?.files ?? []).map(
        (file) => `/home/you/Downloads/${file.name}`,
      );
      if (names.length) void addPaths(names);
    };
    window.addEventListener("dragenter", onEnter);
    window.addEventListener("dragover", onOver);
    window.addEventListener("dragleave", onLeave);
    window.addEventListener("drop", onDrop);
    return () => {
      window.removeEventListener("dragenter", onEnter);
      window.removeEventListener("dragover", onOver);
      window.removeEventListener("dragleave", onLeave);
      window.removeEventListener("drop", onDrop);
    };
  }, [addPaths]);

  useEffect(() => {
    if (api.isTauri) return;
    const onPaste = (event: ClipboardEvent) => {
      const item = Array.from(event.clipboardData?.items ?? []).find((entry) =>
        entry.type.startsWith("image/"),
      );
      if (!item) return;
      const image = item.getAsFile();
      if (!image) return;
      event.preventDefault();
      const reader = new FileReader();
      reader.onload = () => {
        const result = String(reader.result ?? "");
        const data = result.includes(",") ? result.split(",")[1] : "";
        if (!data) return;
        void api
          .saveTempImage(data, image.type || "image/png")
          .then((path) => addPaths([path]))
          .catch((error) =>
            pushToast({
              tone: "error",
              title: "Could not paste the image",
              description: api.errorMessage(error),
            }),
          );
      };
      reader.readAsDataURL(image);
    };
    window.addEventListener("paste", onPaste);
    return () => window.removeEventListener("paste", onPaste);
  }, [addPaths, pushToast]);

  /* ------------------------------------------------------------- render */

  if (booting) {
    return (
      <div className="boot-screen">
        <div className="boot-mark">
          <Mark size={26} />
        </div>
        <div className="boot-copy">
          <h1>Reading the local toolchain</h1>
          <p>Finding the engines already installed on this machine…</p>
        </div>
        <div className="boot-track">
          <span />
        </div>
      </div>
    );
  }

  if (fatal || !catalog || !settings) {
    return (
      <div className="boot-screen is-error">
        <div className="boot-mark">
          <Mark size={26} />
        </div>
        <div className="boot-copy">
          <h1>Could not start</h1>
          <p>{fatal ?? "The catalog did not load."}</p>
        </div>
        <button type="button" className="primary-button" onClick={() => void load()}>
          Try again
        </button>
      </div>
    );
  }

  let page = null;
  if (view === "convert") {
    page = (
      <ConvertPage
        catalog={catalog}
        settings={settings}
        files={files}
        active={active}
        target={target}
        targets={targets}
        perFileTargets={perFileTargets}
        sharedOptions={sharedOptions}
        options={options}
        outputDir={outputDir}
        inspecting={inspecting}
        converting={converting}
        onSelectFile={onSelectFile}
        onTarget={onTarget}
        onOptions={onOptions}
        onApplyAll={applyTargetToAll}
        onOutputDir={(dir) => {
          setOutputDir(dir);
          if (dir) updateSettings({ outputMode: "custom", outputDir: dir });
        }}
        onBrowse={() => void browse()}
        onPaste={() => void paste()}
        onRemove={onRemove}
        onReplace={() => void onReplace()}
        onClear={onClear}
        onConvert={() => void convert()}
        onSaveTo={(folder) => void convert(folder)}
        onHome={goHome}
        onShowFormats={showFormats}
      />
    );
  } else if (view === "formats") {
    page = (
      <FormatsPage
        catalog={catalog}
        refreshing={refreshing}
        onRefresh={() => void refreshCatalog()}
        initialFamily={formatsFamily}
      />
    );
  } else {
    page = (
      <SettingsPage
        settings={settings}
        catalog={catalog}
        onUpdate={updateSettings}
        onRefresh={() => void refreshCatalog()}
        refreshing={refreshing}
        onClearCache={clearCache}
      />
    );
  }

  const fileName = activeFile?.name ?? null;

  return (
    <div className="app-shell">
      <TitleBar
        view={view}
        fileName={fileName}
        theme={resolvedMode}
        onSearch={() => setPaletteOpen(true)}
        onToggleTheme={() =>
          updateSettings({
            theme: resolvedMode === "dark" ? "light" : "dark",
          })
        }
        onHome={goHome}
      />
      <Rail view={view} onChange={(next) => (next === "convert" ? goHome() : navigate(next))} />
      <main className="app-main">
        <div className="page-scroll" key={view}>
          {page}
        </div>
        <ActivityTray
          jobs={jobs}
          catalog={catalog}
          onCancel={(id) => void api.cancelJob(id)}
          onClear={() => void clearJobs()}
          onReveal={(job) => revealOutput(job.output)}
        />
      </main>

      <CommandPalette
        open={paletteOpen}
        catalog={catalog}
        files={files}
        activeFile={active}
        theme={resolvedMode}
        onClose={() => setPaletteOpen(false)}
        onNavigate={navigate}
        onBrowse={() => void browse()}
        onPaste={() => void paste()}
        onTarget={onTarget}
        onConvert={() => void convert()}
        onToggleTheme={() =>
          updateSettings({
            theme: resolvedMode === "dark" ? "light" : "dark",
          })
        }
        onRefresh={() => void refreshCatalog()}
        onClearJobs={() => void clearJobs()}
      />

      {dragging ? (
        <div className="drop-overlay">
          <div className="drop-overlay-inner">
            <span className="drop-overlay-ring">
              <Spinner size={22} />
            </span>
            <h2>Release to inspect</h2>
            <p>
              {catalog.formatCount} formats ready ·{" "}
              {shortPath(settings.outputDir || "~/")}
            </p>
          </div>
        </div>
      ) : null}
    </div>
  );
}

function messageIsEditable(element: HTMLElement | null): boolean {
  return Boolean(element?.isContentEditable);
}
