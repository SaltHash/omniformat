import { Check, ChevronRight, Search } from "lucide-react";
import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { FAMILY_ORDER, familyMeta, targetTag } from "../lib/families";
import type { Family, FormatInfo } from "../lib/types";
import { FormatMark, Kbd } from "./ui";

interface Row {
  format: FormatInfo;
  section: string;
}

export function FormatPicker({
  formats,
  selected,
  recommendedIds,
  sourceFamily,
  sourceId,
  onSelect,
  onClose,
}: {
  formats: FormatInfo[];
  selected: FormatInfo | null;
  recommendedIds: string[];
  sourceFamily: Family | null;
  sourceId: string | null;
  onSelect: (format: FormatInfo) => void;
  onClose: () => void;
}) {
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const rowRefs = useRef(new Map<string, HTMLButtonElement>());

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  const groups = useMemo(() => {
    const needle = query.trim().toLowerCase();
    const matches = formats.filter((format) => {
      if (!needle) return true;
      const meta = familyMeta(format.family);
      return (
        format.label.toLowerCase().includes(needle) ||
        format.id.toLowerCase().includes(needle) ||
        format.extensions.some((ext) => ext.includes(needle)) ||
        meta.label.toLowerCase().includes(needle) ||
        (format.note ?? "").toLowerCase().includes(needle)
      );
    });

    const suggestedIds = new Set(recommendedIds);
    const suggested = recommendedIds
      .map((id) => matches.find((format) => format.id === id))
      .filter((format): format is FormatInfo => Boolean(format));
    const rest = matches.filter((format) => !suggestedIds.has(format.id));
    const result: { family: string; label: string; items: FormatInfo[] }[] = [];
    if (suggested.length) {
      result.push({ family: "suggested", label: "Suggested", items: suggested });
    }
    FAMILY_ORDER.forEach((family) => {
      const items = rest
        .filter((format) => format.family === family)
        .sort((a, b) => {
          const readDiff = Number(b.canWrite) - Number(a.canWrite);
          if (readDiff !== 0) return readDiff;
          return a.label.localeCompare(b.label);
        });
      if (items.length) {
        result.push({
          family,
          label: familyMeta(family).plural,
          items,
        });
      }
    });
    return result;
  }, [formats, query, recommendedIds]);

  const rows = useMemo<Row[]>(
    () => groups.flatMap((group) => group.items.map((format) => ({ format, section: group.label }))),
    [groups],
  );

  useEffect(() => {
    const selectedIndex = rows.findIndex((row) => row.format.id === selected?.id);
    setActiveIndex(selectedIndex >= 0 ? selectedIndex : 0);
  }, [selected?.id, rows.length]);

  useEffect(() => {
    const row = rows[activeIndex];
    if (!row) return;
    rowRefs.current.get(row.format.id)?.scrollIntoView({ block: "nearest" });
  }, [activeIndex, rows]);

  const choose = useCallback(
    (format: FormatInfo) => {
      onSelect(format);
      onClose();
    },
    [onClose, onSelect],
  );

  const onKeyDown = (event: React.KeyboardEvent) => {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setActiveIndex((index) => Math.min(rows.length - 1, index + 1));
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      setActiveIndex((index) => Math.max(0, index - 1));
    } else if (event.key === "Enter") {
      event.preventDefault();
      if (rows[activeIndex]) choose(rows[activeIndex].format);
    } else if (event.key === "Escape") {
      event.preventDefault();
      onClose();
    }
  };

  let lastSection = "";
  let flatIndex = -1;

  return (
    <>
      <div className="picker-scrim" onMouseDown={onClose} />
      <div
        className="format-picker"
        role="dialog"
        aria-label="Choose output format"
        onKeyDown={onKeyDown}
      >
        <div className="picker-search">
          <Search size={14} />
          <input
            ref={inputRef}
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Search formats"
            aria-label="Search output formats"
          />
          <span className="picker-count">{rows.length}</span>
        </div>

        <div className="picker-list" ref={listRef}>
          {groups.length === 0 ? (
            <div className="picker-empty">
              <Search size={18} />
              <p>No format matches “{query}”</p>
              <span>Try an extension or another family.</span>
            </div>
          ) : (
            groups.map((group) => (
              <div key={group.family} className="picker-group">
                <div className="picker-group-label">
                  {group.family === "suggested" ? (
                    <>Suggested</>
                  ) : (
                    <>
                      <span
                        className="family-dot"
                        style={{ background: familyMeta(group.family as Family).color }}
                      />
                      {group.label}
                    </>
                  )}
                </div>
                {group.items.map((format) => {
                  flatIndex += 1;
                  const index = flatIndex;
                  const isActive = index === activeIndex;
                  const isSelected = selected?.id === format.id;
                  const tag = targetTag(sourceFamily, format);
                  return (
                    <button
                      key={format.id}
                      ref={(node) => {
                        if (node) rowRefs.current.set(format.id, node);
                        else rowRefs.current.delete(format.id);
                      }}
                      type="button"
                      className={`picker-row ${isActive ? "is-active" : ""} ${isSelected ? "is-selected" : ""}`}
                      onMouseEnter={() => setActiveIndex(index)}
                      onClick={() => choose(format)}
                    >
                      <FormatMark format={format} size="sm" />
                      <span className="picker-row-main">
                        <span className="picker-row-title">
                          {format.label}
                          {format.lossless ? <span className="tiny-dot" title="Lossless" /> : null}
                        </span>
                        <span className="picker-row-sub">
                          {format.extensions.map((ext) => `.${ext}`).join(" · ")}
                          {format.note ? <span className="picker-note">{format.note}</span> : null}
                        </span>
                      </span>
                      {tag ? <span className="picker-tag">{tag}</span> : null}
                      {isSelected ? (
                        <Check size={14} className="picker-check" />
                      ) : (
                        <ChevronRight size={13} className="picker-arrow" />
                      )}
                    </button>
                  );
                })}
              </div>
            ))
          )}
        </div>

        <div className="picker-footer">
          <span>
            <Kbd>↑</Kbd>
            <Kbd>↓</Kbd> navigate
          </span>
          <span>
            <Kbd>Enter</Kbd> select
          </span>
          <span>
            <Kbd>Esc</Kbd> close
          </span>
          {recommendedIds.length || sourceId ? (
            <span className="picker-footer-right">
              {sourceId ? `${formats.length} targets` : ""}
            </span>
          ) : null}
        </div>
      </div>
    </>
  );
}
