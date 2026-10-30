import {
  Check,
  ChevronDown,
  FolderOpen,
  HardDrive,
  Keyboard,
  Monitor,
  Moon,
  Palette,
  RefreshCw,
  Search,
  ShieldCheck,
  Sparkles,
  Sun,
  Trash2,
  Wrench,
} from "lucide-react";
import { useEffect, useMemo, useState, type ReactNode } from "react";
import * as api from "../lib/api";
import {
  FAMILY_ORDER,
  familyMeta,
  formatById,
} from "../lib/families";
import type { Catalog, Family, FormatInfo, Settings } from "../lib/types";
import {
  FormatMark,
  Kbd,
  PageHeader,
  Segmented,
  Slider,
  Spinner,
  Switch,
} from "../components/ui";

function SettingRow({
  label,
  hint,
  children,
  stacked = false,
}: {
  label: string;
  hint?: string;
  children: ReactNode;
  stacked?: boolean;
}) {
  return (
    <div className={`setting-row ${stacked ? "is-stacked" : ""}`}>
      <div className="setting-label">
        <span>{label}</span>
        {hint ? <small>{hint}</small> : null}
      </div>
      <div className="setting-control">{children}</div>
    </div>
  );
}

function Card({
  icon,
  title,
  description,
  children,
  className = "",
}: {
  icon: ReactNode;
  title: string;
  description?: string;
  children: ReactNode;
  className?: string;
}) {
  return (
    <section className={`settings-card ${className}`}>
      <div className="settings-card-head">
        <span className="settings-icon">{icon}</span>
        <div>
          <h2>{title}</h2>
          {description ? <p>{description}</p> : null}
        </div>
      </div>
      <div className="settings-card-body">{children}</div>
    </section>
  );
}

function filenamePreview(template: string, target = "webp"): string {
  const now = new Date();
  return (
    (template.trim() || "{name}")
      .replaceAll("{name}", "atrium-study")
      .replaceAll("{ext}", "png")
      .replaceAll("{target}", target)
      .replaceAll("{date}", now.toISOString().slice(0, 10))
      .replaceAll(
        "{time}",
        now.toTimeString().slice(0, 8).replaceAll(":", ""),
      ) + `.${target}`
  );
}

function DefaultOutputRow({
  format,
  catalog,
  settings,
  onUpdate,
}: {
  format: FormatInfo;
  catalog: Catalog;
  settings: Settings;
  onUpdate: (patch: Partial<Settings>) => void;
}) {
  const allowed = format.targets
    .map((id) => formatById(catalog, id))
    .filter((item): item is FormatInfo => Boolean(item?.canWrite));
  if (!allowed.length) return null;

  const suggestedId = (catalog.suggestions[format.id] ?? []).find((id) =>
    allowed.some((item) => item.id === id),
  );
  const suggested = formatById(catalog, suggestedId ?? allowed[0].id);
  const defaultTargets = settings.defaultTargets ?? {};
  const override = defaultTargets[format.id];
  const value = allowed.some((item) => item.id === override) ? override : "";
  const suggestedIds = (catalog.suggestions[format.id] ?? []).filter((id) =>
    allowed.some((item) => item.id === id),
  );
  const ordered = [
    ...suggestedIds
      .map((id) => allowed.find((item) => item.id === id))
      .filter((item): item is FormatInfo => Boolean(item)),
    ...allowed
      .filter((item) => !suggestedIds.includes(item.id))
      .sort((a, b) => a.label.localeCompare(b.label)),
  ];

  const change = (next: string) => {
    const defaults = { ...(settings.defaultTargets ?? {}) };
    if (next) defaults[format.id] = next;
    else delete defaults[format.id];
    onUpdate({ defaultTargets: defaults });
  };

  return (
    <div className="default-row">
      <FormatMark format={format} size="xs" />
      <div className="default-source">
        <span className="default-source-name">{format.label}</span>
        <span className="default-source-ext">
          {format.extensions.map((ext) => `.${ext}`).join(" · ")}
        </span>
      </div>
      <span className="select-wrap">
        <select
          className="default-select"
          value={value}
          onChange={(event) => change(event.target.value)}
          aria-label={`Default output for ${format.label}`}
        >
          <option value="">
            Suggested · {suggested?.label ?? allowed[0].label}
          </option>
          {ordered.map((option) => (
            <option key={option.id} value={option.id}>
              {option.label} · .{option.extensions[0] ?? option.id}
            </option>
          ))}
        </select>
        <ChevronDown size={11} className="select-chevron" />
      </span>
    </div>
  );
}

export function SettingsPage({
  settings,
  catalog,
  onUpdate,
  onRefresh,
  refreshing,
  onClearCache,
}: {
  settings: Settings;
  catalog: Catalog;
  onUpdate: (patch: Partial<Settings>) => void;
  onRefresh: () => void;
  refreshing: boolean;
  onClearCache: () => void;
}) {
  const [configPath, setConfigPath] = useState("");
  const [cacheCleared, setCacheCleared] = useState(false);
  const [defaultQuery, setDefaultQuery] = useState("");

  useEffect(() => {
    api
      .configLocation()
      .then(setConfigPath)
      .catch(() => setConfigPath("~/.config/convert"));
  }, []);

  const availableTools = catalog.tools.filter((tool) => tool.available).length;
  const template = settings.filenameTemplate;
  const overrideCount = Object.keys(settings.defaultTargets ?? {}).length;

  const defaultGroups = useMemo(() => {
    const needle = defaultQuery.trim().toLowerCase();
    return FAMILY_ORDER.map((family: Family) => ({
      family,
      meta: familyMeta(family),
      formats: catalog.formats
        .filter(
          (format) =>
            format.family === family &&
            format.canRead &&
            format.targets.length > 0,
        )
        .filter((format) => {
          if (!needle) return true;
          return (
            format.label.toLowerCase().includes(needle) ||
            format.id.toLowerCase().includes(needle) ||
            format.extensions.some((ext) => ext.includes(needle)) ||
            familyMeta(format.family).label.toLowerCase().includes(needle) ||
            (format.note ?? "").toLowerCase().includes(needle)
          );
        })
        .sort((a, b) => a.label.localeCompare(b.label)),
    })).filter((group) => group.formats.length > 0);
  }, [catalog, defaultQuery]);

  const chooseFolder = async () => {
    const folder = await api.pickFolder();
    if (folder) onUpdate({ outputMode: "custom", outputDir: folder });
    else if (settings.outputMode === "custom" && !settings.outputDir) {
      onUpdate({ outputMode: "custom" });
    }
  };

  return (
    <div className="page settings-page">
      <PageHeader
        title="Settings"
        subtitle="Tuned for this machine. Changes save as you make them."
        actions={
          <span className="saved-pill">
            <Check size={12} />
            Auto-saved
          </span>
        }
      />

      <div className="settings-grid">
        <Card
          icon={<FolderOpen size={16} />}
          title="Output"
          description="Where converted files are written."
          className="span-2"
        >
          <SettingRow label="Location" hint="A folder for every conversion, or beside each source">
            <Segmented
              value={settings.outputMode}
              options={[
                { value: "source", label: "Beside source" },
                { value: "custom", label: "Custom folder" },
              ]}
              onChange={(value) => onUpdate({ outputMode: value })}
            />
          </SettingRow>
          {settings.outputMode === "custom" ? (
            <div className="folder-picker">
              <span className="folder-icon">
                <FolderOpen size={14} />
              </span>
              <span className="folder-path" title={settings.outputDir}>
                {settings.outputDir || "No folder selected"}
              </span>
              <button type="button" className="text-button" onClick={chooseFolder}>
                Choose folder
              </button>
            </div>
          ) : (
            <p className="setting-footnote">
              Each output is written next to its source file, never in a hidden cache.
            </p>
          )}
          <SettingRow label="Filename pattern" hint="Tokens are replaced at conversion time" stacked>
            <div className="template-field">
              <input
                className="text-input mono"
                value={template}
                onChange={(event) => onUpdate({ filenameTemplate: event.target.value })}
                placeholder="{name}"
                spellCheck={false}
              />
              <span className="template-preview">{filenamePreview(template)}</span>
            </div>
            <div className="token-row">
              {["{name}", "{ext}", "{target}", "{date}", "{time}"].map((token) => (
                <button
                  key={token}
                  type="button"
                  className="token"
                  onClick={() =>
                    onUpdate({ filenameTemplate: `${template}${token}` })
                  }
                >
                  {token}
                </button>
              ))}
            </div>
          </SettingRow>
          <SettingRow label="Collisions" hint="When a file with that name already exists">
            <Segmented
              value={settings.overwrite}
              options={[
                { value: "rename", label: "Keep both" },
                { value: "skip", label: "Skip" },
                { value: "overwrite", label: "Replace" },
              ]}
              onChange={(value) => onUpdate({ overwrite: value })}
            />
          </SettingRow>
        </Card>

        <Card
          icon={<Sparkles size={16} />}
          title="Default outputs"
          description="The format each source type starts with."
          className="span-2"
        >
          <div className="defaults-toolbar">
            <label className="defaults-search">
              <Search size={12} />
              <input
                value={defaultQuery}
                onChange={(event) => setDefaultQuery(event.target.value)}
                placeholder="Search source types"
                aria-label="Search source types"
              />
            </label>
            <span className="defaults-meta">
              <span>{overrideCount ? `${overrideCount} custom` : "Using suggestions"}</span>
              {overrideCount ? (
                <button
                  type="button"
                  className="text-button is-quiet"
                  onClick={() => onUpdate({ defaultTargets: {} })}
                >
                  Reset all
                </button>
              ) : null}
            </span>
          </div>
          <div className="defaults-list">
            {defaultGroups.length === 0 ? (
              <div className="defaults-empty">
                No source type matches “{defaultQuery}”
              </div>
            ) : (
              defaultGroups.map((group) => (
                <section className="defaults-group" key={group.family}>
                  <h3 className="defaults-group-label">
                    <span
                      className="family-dot"
                      style={{ background: group.meta.color }}
                    />
                    {group.meta.plural}
                  </h3>
                  <div className="defaults-rows">
                    {group.formats.map((format) => (
                      <DefaultOutputRow
                        key={format.id}
                        format={format}
                        catalog={catalog}
                        settings={settings}
                        onUpdate={onUpdate}
                      />
                    ))}
                  </div>
                </section>
              ))
            )}
          </div>
          <p className="setting-footnote">
            New files of each type start with this format. The picker still
            shows every compatible output.
          </p>
        </Card>

        <Card
          icon={<Wrench size={16} />}
          title="Conversion"
          description="Defaults applied to new jobs."
        >
          <div className="setting-slider">
            <div className="setting-slider-head">
              <div className="setting-label">
                <span>Default quality</span>
                <small>Shared by images, video and audio</small>
              </div>
              <span className="quality-badge">{settings.defaultQuality}</span>
            </div>
            <Slider
              value={settings.defaultQuality}
              min={40}
              max={100}
              onChange={(value) => onUpdate({ defaultQuality: value })}
              ariaLabel="Default conversion quality"
            />
          </div>
          <SettingRow label="Parallel jobs" hint="More jobs use more CPU">
            <div className="concurrency-control">
              <Slider
                value={settings.concurrency}
                min={1}
                max={4}
                onChange={(value) => onUpdate({ concurrency: value })}
                ariaLabel="Parallel conversions"
              />
            </div>
          </SettingRow>
          <SettingRow label="Keep metadata" hint="EXIF, colour profiles and comments">
            <Switch
              checked={settings.keepMetadata}
              onChange={(value) => onUpdate({ keepMetadata: value })}
              label="Keep metadata"
            />
          </SettingRow>
        </Card>

        <Card
          icon={<Palette size={16} />}
          title="Appearance"
          description="A quiet interface, in your light."
        >
          <div className="theme-grid">
            {[
              { id: "system" as const, label: "System", icon: <Monitor size={15} /> },
              { id: "dark" as const, label: "Classic dark", icon: <Moon size={15} /> },
              { id: "ink" as const, label: "Ink", icon: <Moon size={15} /> },
              { id: "midnight" as const, label: "Midnight", icon: <Moon size={15} /> },
              { id: "ember" as const, label: "Ember", icon: <Moon size={15} /> },
              { id: "light" as const, label: "Classic light", icon: <Sun size={15} /> },
              { id: "paper" as const, label: "Paper", icon: <Sun size={15} /> },
              { id: "linen" as const, label: "Linen", icon: <Sun size={15} /> },
            ].map((theme) => (
              <button
                key={theme.id}
                type="button"
                className={`theme-card ${settings.theme === theme.id ? "is-active" : ""}`}
                onClick={() => onUpdate({ theme: theme.id })}
              >
                <span className={`theme-swatch theme-${theme.id}`}>
                  <span />
                  <span />
                </span>
                <span className="theme-label">
                  {theme.icon}
                  {theme.label}
                </span>
                {settings.theme === theme.id ? <Check size={13} className="theme-check" /> : null}
              </button>
            ))}
          </div>
          <SettingRow label="Reduce motion" hint="Disable interface animation">
            <Switch
              checked={settings.reduceMotion}
              onChange={(value) => onUpdate({ reduceMotion: value })}
              label="Reduce motion"
            />
          </SettingRow>
          <SettingRow label="Show finished files" hint="Reveal the output in the file manager when a job completes">
            <Switch
              checked={settings.revealOnFinish}
              onChange={(value) => onUpdate({ revealOnFinish: value })}
              label="Show finished files"
            />
          </SettingRow>
        </Card>

        <Card icon={<Keyboard size={16} />} title="Shortcuts" description="Move without reaching for the mouse.">
          <div className="shortcut-list">
            {[
              { keys: ["Ctrl", "K"], label: "Search commands and formats" },
              { keys: ["Ctrl", "O"], label: "Open files" },
              { keys: ["Ctrl", "V"], label: "Paste a file or image" },
              { keys: ["Enter"], label: "Start conversion" },
              { keys: ["Esc"], label: "Close a panel" },
            ].map((row) => (
              <div key={row.label} className="shortcut-row">
                <span>{row.label}</span>
                <span className="shortcut-keys">
                  {row.keys.map((key) => (
                    <Kbd key={key}>{key}</Kbd>
                  ))}
                </span>
              </div>
            ))}
          </div>
        </Card>

        <Card
          icon={<ShieldCheck size={16} />}
          title="Privacy & storage"
          description="Everything stays on this machine."
        >
          <div className="privacy-line">
            <ShieldCheck size={14} />
            <span>No uploads, accounts or telemetry. Conversion is entirely local.</span>
          </div>
          <SettingRow label="Preview cache" hint="Thumbnails and filmstrips">
            <button
              type="button"
              className="secondary-button is-small"
              onClick={() => {
                onClearCache();
                setCacheCleared(true);
                window.setTimeout(() => setCacheCleared(false), 2500);
              }}
            >
              <Trash2 size={12} />
              {cacheCleared ? "Cleared" : "Clear cache"}
            </button>
          </SettingRow>
          <div className="config-location" title={configPath}>
            <HardDrive size={12} />
            <span>{configPath}</span>
          </div>
        </Card>

        <Card
          icon={<Wrench size={16} />}
          title="Toolchain"
          description={`${availableTools} of ${catalog.tools.length} engines available.`}
          className="span-2"
        >
          <div className="toolchain-settings-head">
            <span className="setting-footnote">
              Detection runs locally with each launch. Install a tool to widen format support.
            </span>
            <button
              type="button"
              className="secondary-button is-small"
              onClick={onRefresh}
              disabled={refreshing}
            >
              {refreshing ? <Spinner size={12} /> : <RefreshCw size={12} />}
              {refreshing ? "Scanning" : "Rescan"}
            </button>
          </div>
          <div className="tool-settings-grid">
            {catalog.tools.map((tool) => (
              <div key={tool.id} className={`tool-setting ${tool.available ? "" : "is-missing"}`}>
                <span className={`tool-dot ${tool.available ? "is-live" : ""}`} />
                <div className="tool-setting-main">
                  <strong>{tool.name}</strong>
                  <span>{tool.available ? tool.version ?? tool.path ?? "Detected" : "Not installed"}</span>
                </div>
                <span className="tool-setting-role">{tool.role}</span>
              </div>
            ))}
          </div>
        </Card>
      </div>

      <footer className="settings-about">
        <div>
          <strong>Omniformat</strong>
          <span>Version {catalog.appVersion} · local desktop utility</span>
        </div>
        <span className="about-copy">Made to disappear into the work.</span>
      </footer>
    </div>
  );
}
