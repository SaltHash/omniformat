import { Minus, Moon, Search, Square, Sun, X } from "lucide-react";
import { isTauri } from "../lib/api";
import type { ViewId } from "../lib/types";
import { IconButton, Kbd, Mark } from "./ui";

const VIEW_LABEL: Record<ViewId, string> = {
  convert: "Convert",
  formats: "Formats",
  settings: "Settings",
};

async function windowAction(action: "minimize" | "maximize" | "close") {
  if (!isTauri) return;
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const win = getCurrentWindow();
    if (action === "minimize") await win.minimize();
    if (action === "maximize") await win.toggleMaximize();
    if (action === "close") await win.close();
  } catch {
    /* the browser preview has no window */
  }
}

export function TitleBar({
  view,
  fileName,
  theme,
  onSearch,
  onToggleTheme,
  onHome,
}: {
  view: ViewId;
  fileName?: string | null;
  theme: "dark" | "light";
  onSearch: () => void;
  onToggleTheme: () => void;
  onHome: () => void;
}) {
  return (
    <header
      className="titlebar"
      data-tauri-drag-region
      onDoubleClick={(event) => {
        if ((event.target as HTMLElement).closest("button")) return;
        void windowAction("maximize");
      }}
    >
      <button
        type="button"
        className="titlebar-brand"
        onClick={onHome}
        title="Home"
      >
        <Mark size={17} />
        <span className="titlebar-view">{VIEW_LABEL[view]}</span>
        {view === "convert" && fileName ? (
          <>
            <span className="titlebar-slash">/</span>
            <span className="titlebar-file" title={fileName}>
              {fileName}
            </span>
          </>
        ) : null}
      </button>
      <div className="titlebar-drag" data-tauri-drag-region />

      <div className="titlebar-actions">
        <button
          type="button"
          className="search-trigger"
          onClick={onSearch}
          title="Search commands and formats"
        >
          <Search size={13} strokeWidth={2} />
          <span className="search-trigger-label">Search</span>
          <Kbd>Ctrl K</Kbd>
        </button>
        <IconButton
          label={theme === "dark" ? "Light appearance" : "Dark appearance"}
          onClick={onToggleTheme}
        >
          {theme === "dark" ? <Sun size={15} /> : <Moon size={15} />}
        </IconButton>
        <div className="window-controls">
          <button
            type="button"
            className="window-button"
            aria-label="Minimize"
            title="Minimize"
            onClick={() => void windowAction("minimize")}
          >
            <Minus size={13} />
          </button>
          <button
            type="button"
            className="window-button"
            aria-label="Maximize"
            title="Maximize"
            onClick={() => void windowAction("maximize")}
          >
            <Square size={11} />
          </button>
          <button
            type="button"
            className="window-button is-close"
            aria-label="Close"
            title="Close"
            onClick={() => void windowAction("close")}
          >
            <X size={14} />
          </button>
        </div>
      </div>
    </header>
  );
}
