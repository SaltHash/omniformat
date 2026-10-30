import { Plus, X } from "lucide-react";
import { familyMeta } from "../lib/families";
import type { FileMeta, FormatInfo } from "../lib/types";

export function FileChips({
  files,
  active,
  targets,
  onSelect,
  onRemove,
  onAdd,
  busy,
}: {
  files: FileMeta[];
  active: number;
  targets: Record<string, FormatInfo | null>;
  onSelect: (index: number) => void;
  onRemove: (index: number) => void;
  onAdd: () => void;
  busy: boolean;
}) {
  if (files.length <= 1) return null;
  return (
    <div className="file-chips" aria-label="Selected files">
      <div className="file-chips-scroll">
        {files.map((file, index) => {
          const meta = familyMeta(file.family);
          const Icon = meta.icon;
          const target = targets[file.path] ?? null;
          return (
          <div
            key={`${file.path}-${index}`}
            className={`file-chip ${index === active ? "is-active" : ""}`}
          >
            <button
              type="button"
              className="file-chip-main"
              onClick={() => onSelect(index)}
              title={`${file.path}${target ? ` → .${target.extensions[0] ?? target.id}` : ""}`}
            >
              <Icon size={11} style={{ color: meta.color }} strokeWidth={1.9} />
              <span className="file-chip-name">{file.name}</span>
              {target ? (
                <span className="file-chip-target">
                  .{target.extensions[0] ?? target.id}
                </span>
              ) : null}
            </button>
            <button
              type="button"
              className="file-chip-remove"
              aria-label={`Remove ${file.name}`}
              onClick={() => onRemove(index)}
            >
              <X size={11} />
            </button>
          </div>
          );
        })}
      </div>
      <button type="button" className="file-chip-add" onClick={onAdd} disabled={busy}>
        <Plus size={12} />
        Add
      </button>
    </div>
  );
}
