import {
  ArrowDownToLine,
  ArrowUpFromLine,
  Check,
  RefreshCw,
  Search,
  Sparkles,
  Wrench,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import {
  FAMILIES,
  FAMILY_ORDER,
  familyMeta,
  groupFormatsByFamily,
  shortPath,
} from "../lib/families";
import type { Catalog, Family, FormatInfo } from "../lib/types";
import { FormatMark, PageHeader, Spinner } from "../components/ui";

const CROSS_COPY: Record<string, string> = {
  "video-to-audio": "Video → audio",
  "video-to-frame": "Video → frame",
  "video-to-gif": "Video → GIF",
  "audio-to-video": "Audio → visual",
  "image-to-video": "Image → video",
  "image-to-pdf": "Images → PDF",
  "pdf-to-image": "PDF → image",
  "document-to-pdf": "Documents → PDF",
  "pdf-to-text": "PDF → text",
  "audio-to-waveform": "Audio → waveform",
};

function FormatTile({ format }: { format: FormatInfo }) {
  const meta = familyMeta(format.family);
  return (
    <div
      className={`format-tile ${!format.canWrite ? "is-readonly" : ""}`}
      title={`${format.label} · ${format.extensions.map((e) => `.${e}`).join(", ")}`}
    >
      <FormatMark format={format} size="sm" />
      <div className="format-tile-text">
        <strong>{format.label}</strong>
        <span>{format.extensions.map((ext) => `.${ext}`).join(" ")}</span>
      </div>
      <div className="format-tile-badges">
        {format.lossless ? <span className="lossless-dot" title="Lossless" /> : null}
        {format.canRead ? (
          <ArrowDownToLine
            size={11}
            className="io-badge"
            aria-label="Can read"
          />
        ) : null}
        {format.canWrite ? (
          <ArrowUpFromLine
            size={11}
            className="io-badge is-write"
            style={{ color: meta.color }}
            aria-label="Can write"
          />
        ) : null}
      </div>
    </div>
  );
}

export function FormatsPage({
  catalog,
  refreshing,
  onRefresh,
  initialFamily,
}: {
  catalog: Catalog;
  refreshing: boolean;
  onRefresh: () => void;
  initialFamily?: Family | null;
}) {
  const [query, setQuery] = useState("");
  const [family, setFamily] = useState<Family | "all">("all");
  const searchRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (initialFamily) setFamily(initialFamily);
  }, [initialFamily]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (
        event.key === "/" &&
        !(event.target instanceof HTMLInputElement) &&
        !(event.target instanceof HTMLTextAreaElement)
      ) {
        event.preventDefault();
        searchRef.current?.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return catalog.formats.filter((format) => {
      if (family !== "all" && format.family !== family) return false;
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
  }, [catalog.formats, family, query]);

  const groups = useMemo(() => groupFormatsByFamily(filtered), [filtered]);
  const familyCounts = useMemo(() => {
    const counts = new Map<string, number>();
    catalog.formats.forEach((format) =>
      counts.set(format.family, (counts.get(format.family) ?? 0) + 1),
    );
    return counts;
  }, [catalog.formats]);

  const availableTools = catalog.tools.filter((tool) => tool.available).length;
  const readable = catalog.formats.filter((format) => format.canRead).length;
  const writable = catalog.formats.filter((format) => format.canWrite).length;

  return (
    <div className="page formats-page">
      <PageHeader
        title="Formats"
        subtitle="Every format the local toolchain can read and write, discovered at launch."
        actions={
          <button type="button" className="secondary-button is-small" onClick={onRefresh} disabled={refreshing}>
            {refreshing ? <Spinner size={13} /> : <RefreshCw size={13} />}
            {refreshing ? "Scanning" : "Refresh"}
          </button>
        }
      />

      <div className="format-hero">
        <div className="format-stats">
          <div>
            <strong>{catalog.formatCount}</strong>
            <span>formats</span>
          </div>
          <div>
            <strong>{FAMILY_ORDER.filter((id) => (familyCounts.get(id) ?? 0) > 0).length}</strong>
            <span>families</span>
          </div>
          <div>
            <strong>{readable}</strong>
            <span>readable</span>
          </div>
          <div>
            <strong>{writable}</strong>
            <span>writable</span>
          </div>
          <div>
            <strong>{availableTools}</strong>
            <span>tools live</span>
          </div>
        </div>
        <div className="cross-list">
          <span className="eyebrow">
            <Sparkles size={11} />
            Cross-format
          </span>
          <div className="cross-chips">
            {catalog.cross.map((item) => (
              <span key={item} className="cross-chip">
                <Check size={10} />
                {CROSS_COPY[item] ?? item.replaceAll("-", " ")}
              </span>
            ))}
          </div>
        </div>
      </div>

      <div className="formats-toolbar">
        <div className="search-field">
          <Search size={14} />
          <input
            ref={searchRef}
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Search formats, extensions, families"
            aria-label="Search formats"
          />
          {!query ? <kbd className="kbd">/</kbd> : null}
        </div>
        <div className="filter-chips">
          <button
            type="button"
            className={family === "all" ? "is-active" : ""}
            onClick={() => setFamily("all")}
          >
            All
            <span>{catalog.formatCount}</span>
          </button>
          {FAMILY_ORDER.filter((id) => (familyCounts.get(id) ?? 0) > 0).map((id) => {
            const meta = FAMILIES[id];
            return (
              <button
                key={id}
                type="button"
                className={family === id ? "is-active" : ""}
                onClick={() => setFamily(id)}
              >
                <span className="family-dot" style={{ background: meta.color }} />
                {meta.plural}
                <span>{familyCounts.get(id)}</span>
              </button>
            );
          })}
        </div>
      </div>

      {groups.length === 0 ? (
        <div className="formats-empty">
          <Search size={20} />
          <h3>No formats match “{query}”</h3>
          <p>Try searching for an extension such as “webp” or a family like “audio”.</p>
        </div>
      ) : (
        <div className="format-sections">
          {groups.map((group) => {
            const meta = FAMILIES[group.family];
            const Icon = meta.icon;
            return (
              <section key={group.family} className="format-section">
                <div className="format-section-head">
                  <span
                    className="family-icon"
                    style={{ color: meta.color, borderColor: `${meta.color}33` }}
                  >
                    <Icon size={15} />
                  </span>
                  <div>
                    <h2>{meta.plural}</h2>
                    <p>{meta.blurb}</p>
                  </div>
                  <span className="family-count">{group.items.length}</span>
                </div>
                <div className="format-grid">
                  {group.items.map((format) => (
                    <FormatTile key={format.id} format={format} />
                  ))}
                </div>
              </section>
            );
          })}
        </div>
      )}

      <section className="toolchain">
        <div className="toolchain-head">
          <div className="toolchain-title">
            <span className="toolchain-icon">
              <Wrench size={15} />
            </span>
            <div>
              <h2>Local toolchain</h2>
              <p>Conversions delegate to these installed engines. Nothing is uploaded.</p>
            </div>
          </div>
          <span className="toolchain-count">
            {availableTools}/{catalog.tools.length} available
          </span>
        </div>
        <div className="tool-grid">
          {catalog.tools.map((tool) => (
            <div key={tool.id} className={`tool-card ${tool.available ? "" : "is-missing"}`}>
              <div className="tool-card-top">
                <span className={`tool-dot ${tool.available ? "is-live" : ""}`} />
                <strong>{tool.name}</strong>
                <span className="tool-role">{tool.role}</span>
              </div>
              <div className="tool-detail" title={tool.path ?? undefined}>
                {tool.available ? tool.version ?? shortPath(tool.path) : "Not found"}
              </div>
            </div>
          ))}
        </div>
      </section>
    </div>
  );
}
