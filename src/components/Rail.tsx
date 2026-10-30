import {
  ArrowRightLeft,
  LayoutGrid,
  SlidersHorizontal,
  type LucideIcon,
} from "lucide-react";
import type { ViewId } from "../lib/types";

const ITEMS: { id: ViewId; label: string; icon: LucideIcon }[] = [
  { id: "convert", label: "Convert", icon: ArrowRightLeft },
  { id: "formats", label: "Formats", icon: LayoutGrid },
  { id: "settings", label: "Settings", icon: SlidersHorizontal },
];

export function Rail({
  view,
  onChange,
}: {
  view: ViewId;
  onChange: (view: ViewId) => void;
}) {
  return (
    <nav className="rail" aria-label="Primary">
      <div className="rail-main">
        {ITEMS.map((item) => {
          const Icon = item.icon;
          const active = view === item.id;
          return (
            <button
              key={item.id}
              type="button"
              className={`rail-button ${active ? "is-active" : ""}`}
              aria-label={item.label}
              aria-current={active ? "page" : undefined}
              onClick={() => onChange(item.id)}
            >
              <Icon size={18} strokeWidth={active ? 2 : 1.7} />
              <span className="rail-tip">{item.label}</span>
            </button>
          );
        })}
      </div>
      <div className="rail-bottom">
        <div className="local-badge" title="Every conversion runs on this machine">
          <span className="local-dot" />
          <span>Local</span>
        </div>
      </div>
    </nav>
  );
}
