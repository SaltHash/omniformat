import {
  ArrowRight,
  ChevronDown,
  Clipboard,
  FolderOpen,
  LayoutGrid,
  Lock,
  MousePointer2,
  ShieldCheck,
  Sparkles,
  TriangleAlert,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import * as api from "../lib/api";
import {
  FAMILY_ORDER,
  familyMeta,
  fileDetail,
  formatById,
  modeLabel,
  shortPath,
  type FamilyMeta,
} from "../lib/families";
import type {
  Catalog,
  Family,
  FileMeta,
  FilmFrame,
  FormatInfo,
  Options,
  Settings,
} from "../lib/types";
import { FileChips } from "../components/FileChips";
import { Filmstrip } from "../components/Filmstrip";
import { FormatPicker } from "../components/FormatPicker";
import { OptionsPanel } from "../components/OptionsPanel";
import { PreviewCard } from "../components/PreviewCard";
import { FormatMark, Kbd, PageHeader, Spinner } from "../components/ui";

export interface ConvertPageProps {
  catalog: Catalog;
  settings: Settings;
  files: FileMeta[];
  active: number;
  target: FormatInfo | null;
  targets: FormatInfo[];
  perFileTargets: Record<string, FormatInfo | null>;
  sharedOptions: boolean;
  options: Options;
  outputDir: string | null;
  inspecting: boolean;
  converting: boolean;
  onSelectFile: (index: number) => void;
  onTarget: (format: FormatInfo) => void;
  onOptions: (patch: Partial<Options>) => void;
  onApplyAll: () => void;
  onOutputDir: (dir: string | null) => void;
  onBrowse: () => void;
  onPaste: () => void;
  onRemove: (index: number) => void;
  onReplace: () => void;
  onClear: () => void;
  onConvert: () => void;
  onSaveTo: (folder: string) => void;
  onHome: () => void;
  onShowFormats: (family?: Family) => void;
}

function familyCounts(catalog: Catalog): { meta: FamilyMeta; count: number }[] {
  return FAMILY_ORDER.map((family) => ({
    meta: familyMeta(family),
    count: catalog.formats.filter((format) => format.family === family).length,
  })).filter((entry) => entry.count > 0);
}

function templateName(file: FileMeta, target: FormatInfo, settings: Settings): string {
  const stem = file.name.replace(/\.[^.]+$/, "");
  const ext = file.name.split(".").pop()?.toLowerCase() ?? "";
  const now = new Date();
  const date = now.toISOString().slice(0, 10);
  const time = now.toTimeString().slice(0, 8).replace(/:/g, "");
  const template = settings.filenameTemplate.trim() || "{name}";
  const base = template
    .replaceAll("{name}", stem)
    .replaceAll("{ext}", ext)
    .replaceAll("{target}", target.id)
    .replaceAll("{date}", date)
    .replaceAll("{time}", time)
    .replaceAll("/", "-");
  return `${base}.${target.extensions[0] ?? target.id}`;
}

function DropStage({
  catalog,
  onBrowse,
  onPaste,
  onFamily,
  inspecting,
}: {
  catalog: Catalog;
  onBrowse: () => void;
  onPaste: () => void;
  onFamily: (family: Family) => void;
  inspecting: boolean;
}) {
  const counts = familyCounts(catalog);
  return (
    <section className="drop-stage">
      <div className="stage-grid" aria-hidden="true" />
      <div className="stage-glow" aria-hidden="true" />
      <span className="stage-corner is-tl" aria-hidden="true" />
      <span className="stage-corner is-br" aria-hidden="true" />
      <div className="drop-inner">
        <span className="eyebrow drop-eyebrow">
          <span className="eyebrow-line" />
          {catalog.formatCount} formats · {counts.length} families
          <span className="eyebrow-line" />
        </span>
        <h1 className="drop-title">
          Drop <em>anything</em>
          <span className="accent-dot">.</span>
        </h1>
        <p className="drop-sub">
          Images, video, audio, documents, archives. One quiet workspace for
          every conversion — all of it on this machine.
        </p>
        <div className="drop-actions">
          <button type="button" className="primary-button" onClick={onBrowse}>
            <FolderOpen size={16} />
            Browse files
          </button>
          <button type="button" className="secondary-button" onClick={onPaste}>
            <Clipboard size={15} />
            Paste file
            <Kbd>Ctrl V</Kbd>
          </button>
        </div>
        <div className="family-strip">
          {counts.map(({ meta, count }) => {
            const Icon = meta.icon;
            return (
              <button
                key={meta.id}
                type="button"
                className="family-chip"
                onClick={() => onFamily(meta.id)}
                title={`Browse ${meta.plural.toLowerCase()} formats`}
              >
                <Icon size={13} style={{ color: meta.color }} strokeWidth={1.8} />
                <span>{meta.plural}</span>
                <small>{count}</small>
              </button>
            );
          })}
        </div>
      </div>
      <div className="stage-foot">
        <span>
          <MousePointer2 size={12} />
          Drag from any window, or paste a copied file
        </span>
        <span>
          <ShieldCheck size={13} />
          Nothing leaves this device
        </span>
      </div>
      {inspecting ? (
        <div className="stage-reading">
          <Spinner size={18} />
          <span>Reading file…</span>
        </div>
      ) : null}
    </section>
  );
}

function Warning({ children }: { children: React.ReactNode }) {
  return (
    <div className="inline-warning">
      <TriangleAlert size={14} />
      <span>{children}</span>
    </div>
  );
}

export function ConvertPage(props: ConvertPageProps) {
  const {
    catalog,
    settings,
    files,
    active,
    target,
    targets,
    perFileTargets,
    sharedOptions,
    options,
    outputDir,
    inspecting,
    converting,
    onSelectFile,
    onTarget,
    onOptions,
    onApplyAll,
    onOutputDir,
    onBrowse,
    onPaste,
    onRemove,
    onReplace,
    onClear,
    onConvert,
    onSaveTo,
    onHome,
    onShowFormats,
  } = props;

  const file = files[active] ?? files[0] ?? null;
  const sourceFormat = formatById(catalog, file?.formatId);
  const [pickerOpen, setPickerOpen] = useState(false);
  const recommendations = file
    ? catalog.suggestions[file.formatId] ?? []
    : [];
  const recommendedId = recommendations[0] ?? null;
  const isRecommended = Boolean(target && recommendedId === target.id);

  const needsFrame =
    file?.family === "video" &&
    target?.family === "image" &&
    target.id !== "gif";
  const gifTarget = file?.family === "video" && target?.id === "gif";

  const [frames, setFrames] = useState<FilmFrame[]>([]);
  const [filmLoading, setFilmLoading] = useState(false);
  const [framePreview, setFramePreview] = useState<string | null>(null);
  const filmCache = useRef(new Map<string, FilmFrame[]>());
  const frameTime = options.frameTime ?? 1;

  useEffect(() => {
    if (!file || !needsFrame) {
      setFrames([]);
      setFramePreview(null);
      return;
    }
    const cached = filmCache.current.get(file.path);
    if (cached) {
      setFrames(cached);
      return;
    }
    let cancelled = false;
    setFilmLoading(true);
    api
      .getFilmstrip(file.path)
      .then((items) => {
        if (cancelled) return;
        filmCache.current.set(file.path, items);
        setFrames(items);
      })
      .catch(() => {
        if (!cancelled) setFrames([]);
      })
      .finally(() => {
        if (!cancelled) setFilmLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [file, needsFrame]);

  useEffect(() => {
    if (!file || !needsFrame) {
      setFramePreview(null);
      return;
    }
    let cancelled = false;
    const timer = window.setTimeout(() => {
      api
        .getFrame(file.path, frameTime)
        .then((image) => {
          if (!cancelled) setFramePreview(image);
        })
        .catch(() => {
          /* keep the filmstrip thumb */
        });
    }, 220);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [file, needsFrame, frameTime]);

  useEffect(() => {
    setFramePreview(null);
  }, [file?.path]);

  const stagedTargets = files.map(
    (item) => perFileTargets[item.path] ?? null,
  );
  const sharedTarget =
    stagedTargets.length > 0 &&
    stagedTargets.every((item) => item?.id === target?.id);
  const mergedPdf =
    files.length > 1 &&
    sharedTarget &&
    sharedOptions &&
    target?.id === "pdf" &&
    files.every((item) => item.family === "image" || item.family === "vector");
  const mergedVideo =
    files.length > 1 &&
    sharedTarget &&
    sharedOptions &&
    target?.family === "video" &&
    files.every((item) => item.family === "image");
  const canApplyAll = Boolean(
    target &&
      files.length > 1 &&
      files.every((item) =>
        formatById(catalog, item.formatId)?.targets.includes(target.id),
      ),
  );

  const outputName =
    file && target ? templateName(file, target, settings) : "";

  const directoryLabel = outputDir
    ? shortPath(outputDir)
    : settings.outputMode === "custom" && settings.outputDir
      ? shortPath(settings.outputDir)
      : mergedPdf
        ? "Same folder as the first image"
        : "Same folder as the source";

  const counts = familyCounts(catalog);
  const statusLabel = `${catalog.formatCount} formats ready`;

  const pickTime = (time: number) => {
    onOptions({ frameTime: time });
    const nearest = frames.reduce<FilmFrame | null>((best, frame) => {
      if (!best) return frame;
      return Math.abs(frame.time - time) < Math.abs(best.time - time) ? frame : best;
    }, null);
    if (nearest) setFramePreview(nearest.image);
  };

  const headerActions = (
    <>
      <button
        type="button"
        className="status-pill"
        onClick={() => onShowFormats()}
        title="Browse every supported format"
      >
        <span className="status-dot" />
        {statusLabel}
        <LayoutGrid size={12} />
      </button>
    </>
  );

  return (
    <div className="convert-page page">
      <PageHeader
        title="Convert"
        onTitleClick={file ? onHome : undefined}
        subtitle={
          files.length
            ? files.length === 1
              ? `1 file ready · ${targets.length} possible outputs`
              : `${files.length} files ready · each keeps its own format`
            : `A precision converter for every kind of file. ${counts.length} families, one workspace.`
        }
        actions={headerActions}
      />

      {!file ? (
        <DropStage
          catalog={catalog}
          onBrowse={onBrowse}
          onPaste={onPaste}
          onFamily={(family) => onShowFormats(family)}
          inspecting={inspecting}
        />
      ) : (
        <div className="workspace">
          <FileChips
            files={files}
            active={active}
            targets={perFileTargets}
            onSelect={onSelectFile}
            onRemove={onRemove}
            onAdd={onBrowse}
            busy={inspecting}
          />
          <div className={`workspace-grid ${needsFrame ? "has-filmstrip" : ""}`}>
            <div className="preview-pane">
              <PreviewCard
                file={file}
                format={sourceFormat}
                framePreview={needsFrame ? framePreview : null}
                frameTime={needsFrame ? frameTime : null}
                onReplace={onReplace}
                onRemove={() => onRemove(active)}
              />
              {needsFrame ? (
                <Filmstrip
                  frames={frames}
                  duration={file.duration ?? 0}
                  time={frameTime}
                  loading={filmLoading}
                  onChange={pickTime}
                />
              ) : null}
              {gifTarget ? (
                <div className="preview-hint">
                  <Sparkles size={13} />
                  An animated GIF is built from the entire clip. Adjust the frame
                  rate and width on the right.
                </div>
              ) : null}
            </div>

            <aside className="target-panel">
              <div className="target-head">
                <span className="eyebrow">Output format</span>
                <span className="target-head-side">
                  {files.length > 1 ? (
                    <button
                      type="button"
                      className="text-button is-quiet"
                      onClick={onApplyAll}
                      disabled={!canApplyAll}
                      title={
                        canApplyAll
                          ? `Use ${target?.label} for every selected file`
                          : "This format is not available for every file"
                      }
                    >
                      Format for all
                    </button>
                  ) : null}
                  <span className="target-head-meta">
                    {files.length > 1
                      ? `${active + 1} of ${files.length}`
                      : fileDetail(file)}
                  </span>
                </span>
              </div>

              <div className="target-block">
                <button
                  type="button"
                  className="target-select"
                  onClick={() => setPickerOpen(true)}
                  disabled={targets.length === 0}
                >
                  <FormatMark format={target} size="lg" />
                  <span className="target-info">
                    <span className="target-name">
                      {target?.label ?? "No output"}
                      {isRecommended ? <span className="recommend-tag">Recommended</span> : null}
                    </span>
                    <span className="target-hint">
                      {target
                        ? `${target.extensions.map((ext) => `.${ext}`).join(" · ")}${target.note ? ` · ${target.note}` : ""}`
                        : "No compatible format"}
                    </span>
                  </span>
                  <ChevronDown size={16} className="target-chevron" />
                </button>
                {pickerOpen && target ? (
                  <FormatPicker
                    formats={targets}
                    selected={target}
                    recommendedIds={recommendations}
                    sourceFamily={file.family === "other" ? null : file.family}
                    sourceId={file.formatId}
                    onSelect={onTarget}
                    onClose={() => setPickerOpen(false)}
                  />
                ) : null}
              </div>

              <div className="mode-row">
                <span className="mode-chip">
                  {modeLabel(file.family, target)}
                </span>
                <span className="mode-line" />
                <span className="mode-count">
                  {sourceFormat?.label ?? file.formatLabel} → {target?.label ?? "—"}
                </span>
              </div>

              {targets.length === 0 ? (
                <Warning>
                  This file type has no available output on the current toolchain.
                </Warning>
              ) : null}

              <div className="panel-divider" />

              <div className="options-section">
                <div className="options-head">
                  <span className="eyebrow">Adjustments</span>
                  {files.length > 1 ? (
                    <span className="options-scope">This file only</span>
                  ) : null}
                </div>
                {target ? (
                  <OptionsPanel
                    file={file}
                    target={target}
                    options={options}
                    onChange={onOptions}
                  />
                ) : null}
              </div>

              <div className="panel-spacer" />

              <div className="destination">
                <div className="destination-head">
                  <span className="eyebrow">Save to</span>
                  <button
                    type="button"
                    className="text-button"
                    onClick={async () => {
                      const folder = await api.pickFolder();
                      if (folder) onOutputDir(folder);
                    }}
                  >
                    <FolderOpen size={12} />
                    Change
                  </button>
                </div>
                <div className="destination-row">
                  <span className="destination-path" title={outputDir ?? settings.outputDir}>
                    {directoryLabel}
                  </span>
                  {outputDir ? (
                    <button
                      type="button"
                      className="text-button is-quiet"
                      onClick={() => onOutputDir(null)}
                    >
                      Reset
                    </button>
                  ) : null}
                </div>
                <div className="filename-preview" title={outputName}>
                  {outputName}
                </div>
              </div>

              <div className="convert-actions">
              <button
                type="button"
                className="convert-button"
                onClick={onConvert}
                disabled={!target || converting || targets.length === 0}
              >
                {converting ? <Spinner size={16} /> : <ArrowRight size={17} />}
                <span>
                  {converting
                    ? "Converting…"
                    : mergedPdf
                      ? `Merge ${files.length} images to PDF`
                      : files.length > 1
                        ? sharedTarget
                          ? `Convert ${files.length} files to ${target?.label ?? ""}`
                          : `Convert ${files.length} files`
                        : `Save to ${target?.label ?? ""}`}
                </span>
              </button>
              <button
                type="button"
                className="convert-button is-ghost"
                onClick={async () => {
                  const folder = await api.pickFolder();
                  if (folder) onSaveTo(folder);
                }}
                disabled={!target || converting || targets.length === 0}
                title="Save to a chosen folder"
                aria-label="Save to a chosen folder"
              >
                <FolderOpen size={17} />
              </button>
              </div>
              <div className="convert-note">
                <Lock size={11} />
                Local only ·{" "}
                {mergedVideo || mergedPdf
                  ? "one combined file"                    : files.length > 1 && !sharedTarget
                      ? "each file keeps its format"
                      : outputDir || settings.outputMode === "custom"
                        ? "saved to the chosen folder"
                        : "saved beside the source"}
              </div>
            </aside>
          </div>
        </div>
      )}

    </div>
  );
}
