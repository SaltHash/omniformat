import {
  ArrowRight,
  ArrowRightLeft,
  Clipboard,
  FolderOpen,
  LayoutGrid,
  Moon,
  RefreshCw,
  Search,
  Settings2,
  Sun,
  Trash2,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { formatById, familyMeta, modeLabel } from "../lib/families";
import type {
  Catalog,
  Family,
  FileMeta,
  FormatInfo,
  ViewId,
} from "../lib/types";
import { FormatMark, Kbd } from "./ui";

interface Item {
  id: string;
  label: string;
  hint?: string;
  section: string;
  icon: ReactNode;
  shortcut?: string;
  disabled?: boolean;
  action: () => void;
}

export function CommandPalette({
  open,
  catalog,
  files,
  activeFile,
  theme,
  onClose,
  onNavigate,
  onBrowse,
  onPaste,
  onTarget,
  onConvert,
  onToggleTheme,
  onRefresh,
  onClearJobs,
}: {
  open: boolean;
  catalog: Catalog | null;
  files: FileMeta[];
  activeFile: number;
  theme: "dark" | "light";
  onClose: () => void;
  onNavigate: (view: ViewId) => void;
  onBrowse: () => void;
  onPaste: () => void;
  onTarget: (format: FormatInfo) => void;
  onConvert: () => void;
  onToggleTheme: () => void;
  onRefresh: () => void;
  onClearJobs: () => void;
}) {
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (open) {
      setQuery("");
      setActive(0);
      window.setTimeout(() => inputRef.current?.focus(), 30);
    }
  }, [open]);

  const targetFormats = useMemo(() => {
    const file = files[activeFile] ?? files[0];
    if (!catalog || !file) return [];
    const allowed = formatById(catalog, file.formatId)?.targets ?? [];
    const recommended = catalog.suggestions[file.formatId] ?? [];
    const rank = (id: string) => {
      const index = recommended.indexOf(id);
      return index === -1 ? recommended.length + 1 : index;
    };
    return catalog.formats
      .filter((format) => format.canWrite && allowed.includes(format.id))
      .sort(
        (a, b) => rank(a.id) - rank(b.id) || a.label.localeCompare(b.label),
      );
  }, [activeFile, catalog, files]);

  const items = useMemo<Item[]>(() => {
    const base: Item[] = [
      {
        id: "nav-convert",
        label: "Go to Convert",
        hint: "Open the conversion workspace",
        section: "Navigate",
        icon: <ArrowRightLeft size={15} />,
        action: () => onNavigate("convert"),
      },
      {
        id: "nav-formats",
        label: "Go to Formats",
        hint: "Browse the format library",
        section: "Navigate",
        icon: <LayoutGrid size={15} />,
        action: () => onNavigate("formats"),
      },
      {
        id: "nav-settings",
        label: "Go to Settings",
        hint: "Output, quality and appearance",
        section: "Navigate",
        icon: <Settings2 size={15} />,
        action: () => onNavigate("settings"),
      },
      {
        id: "action-browse",
        label: "Browse files",
        hint: "Choose one or more files",
        section: "Actions",
        icon: <FolderOpen size={15} />,
        shortcut: "Ctrl O",
        action: onBrowse,
      },
      {
        id: "action-paste",
        label: "Paste from clipboard",
        hint: "Files and screenshots",
        section: "Actions",
        icon: <Clipboard size={15} />,
        shortcut: "Ctrl V",
        action: onPaste,
      },
      {
        id: "action-convert",
        label: "Convert now",
        hint: files.length ? `${files.length} file${files.length === 1 ? "" : "s"} selected` : "Choose a file first",
        section: "Actions",
        icon: <ArrowRight size={15} />,
        shortcut: "Enter",
        disabled: !files.length,
        action: onConvert,
      },
      {
        id: "action-theme",
        label: `Switch to ${theme === "dark" ? "light" : "dark"} appearance`,
        hint: "Change the interface theme",
        section: "Actions",
        icon: theme === "dark" ? <Sun size={15} /> : <Moon size={15} />,
        action: onToggleTheme,
      },
      {
        id: "action-refresh",
        label: "Refresh format library",
        hint: "Rescan the local toolchain",
        section: "Actions",
        icon: <RefreshCw size={15} />,
        action: onRefresh,
      },
      {
        id: "action-clear",
        label: "Clear finished jobs",
        hint: "Tidy the activity tray",
        section: "Actions",
        icon: <Trash2 size={15} />,
        action: onClearJobs,
      },
    ];

    const formats: Item[] = targetFormats.map((format) => ({
      id: `format-${format.id}`,
      label: `Convert to ${format.label}`,
      hint: modeLabel(
        (files[activeFile] ?? files[0])?.family as Family | undefined,
        format,
      ),
      section: "Output formats",
      icon: <FormatMark format={format} size="xs" />,
      action: () => onTarget(format),
    }));

    return [...base, ...formats];
  }, [
    activeFile,
    files,
    onBrowse,
    onClearJobs,
    onConvert,
    onNavigate,
    onPaste,
    onRefresh,
    onTarget,
    onToggleTheme,
    targetFormats,
    theme,
  ]);

  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();
    const list = needle
      ? items.filter((item) => {
          const format = item.id.startsWith("format-")
            ? targetFormats.find((f) => f.id === item.id.slice(7))
            : null;
          return (
            item.label.toLowerCase().includes(needle) ||
            (item.hint ?? "").toLowerCase().includes(needle) ||
            (format?.extensions ?? []).some((ext) => ext.includes(needle)) ||
            (format ? familyMeta(format.family).label.toLowerCase().includes(needle) : false)
          );
        })
      : items.filter((item) => !item.id.startsWith("format-") || targetFormats.indexOf(
          targetFormats.find((f) => f.id === item.id.slice(7)) as FormatInfo,
        ) < 14);
    return list;
  }, [items, query, targetFormats]);

  useEffect(() => {
    setActive(0);
  }, [query]);

  if (!open) return null;

  const run = (item: Item) => {
    if (item.disabled) return;
    onClose();
    item.action();
  };

  const onKeyDown = (event: React.KeyboardEvent) => {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setActive((index) => Math.min(filtered.length - 1, index + 1));
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      setActive((index) => Math.max(0, index - 1));
    } else if (event.key === "Enter") {
      event.preventDefault();
      if (filtered[active]) run(filtered[active]);
    } else if (event.key === "Escape") {
      event.preventDefault();
      onClose();
    }
  };

  let section = "";

  return (
    <div className="palette-overlay" onMouseDown={onClose}>
      <div
        className="palette"
        role="dialog"
        aria-label="Command palette"
        onMouseDown={(event) => event.stopPropagation()}
        onKeyDown={onKeyDown}
      >
        <div className="palette-search">
          <Search size={16} />
          <input
            ref={inputRef}
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Type a command or format…"
            aria-label="Search commands"
          />
          <Kbd>Esc</Kbd>
        </div>
        <div className="palette-list" ref={listRef}>
          {filtered.length === 0 ? (
            <div className="palette-empty">
              <Search size={18} />
              <p>No command matches “{query}”</p>
            </div>
          ) : (
            filtered.map((item, index) => {
              const header = item.section !== section ? item.section : null;
              section = item.section;
              return (
                <div key={item.id}>
                  {header ? <div className="palette-section">{header}</div> : null}
                  <button
                    type="button"
                    className={`palette-item ${index === active ? "is-active" : ""} ${item.disabled ? "is-disabled" : ""}`}
                    onMouseEnter={() => setActive(index)}
                    onClick={() => run(item)}
                  >
                    <span className="palette-item-icon">{item.icon}</span>
                    <span className="palette-item-main">
                      <span>{item.label}</span>
                      {item.hint ? <small>{item.hint}</small> : null}
                    </span>
                    {item.shortcut ? <Kbd>{item.shortcut}</Kbd> : null}
                  </button>
                </div>
              );
            })
          )}
        </div>
        <div className="palette-footer">
          <span>
            <Kbd>↑</Kbd>
            <Kbd>↓</Kbd> navigate
          </span>
          <span>
            <Kbd>Enter</Kbd> run
          </span>
          <span className="palette-brand">
            {catalog ? `${catalog.formatCount} formats · local only` : "local only"}
          </span>
        </div>
      </div>
    </div>
  );
}
