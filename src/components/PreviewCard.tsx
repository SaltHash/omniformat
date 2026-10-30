import { RefreshCw, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import { fileDetail, familyMeta, humanSize, timecode } from "../lib/families";
import type { FileMeta, FormatInfo } from "../lib/types";
import { FormatMark } from "./ui";

export function PreviewCard({
  file,
  format,
  framePreview,
  frameTime,
  onReplace,
  onRemove,
}: {
  file: FileMeta;
  format: FormatInfo | null;
  framePreview: string | null;
  frameTime: number | null;
  onReplace: () => void;
  onRemove: () => void;
}) {
  const meta = familyMeta(file.family);
  const Icon = meta.icon;
  const [failed, setFailed] = useState(false);
  const [videoSrc, setVideoSrc] = useState<string | null>(null);
  useEffect(() => setFailed(false), [file.path, file.thumbnail, framePreview]);
  useEffect(() => {
    setVideoSrc(null);
    if (file.family !== "video") return;
    let cancelled = false;
    void import("@tauri-apps/api/core").then(({ convertFileSrc }) => {
      if (!cancelled) setVideoSrc(convertFileSrc(file.path));
    });
    return () => {
      cancelled = true;
    };
  }, [file.path, file.family]);

  const video = file.family === "video";
  const audio = file.family === "audio";
  const hasImage = Boolean(framePreview || file.thumbnail) && !failed;
  const isFrame = video && frameTime != null;

  return (
    <section className="preview-card" aria-label="Selected file">
      <div
        className={`preview-media ${hasImage ? "has-image" : ""} ${video ? "is-video" : ""} ${audio ? "is-audio" : ""}`}
      >
        {video && videoSrc ? (
          <video
            key={videoSrc}
            src={videoSrc}
            poster={framePreview ?? file.thumbnail ?? undefined}
            controls
            preload="metadata"
            className="preview-video"
          />
        ) : hasImage ? (
          <img
            src={framePreview ?? file.thumbnail ?? ""}
            alt=""
            onError={() => setFailed(true)}
          />
        ) : (
          <div className="preview-fallback" style={{ "--mark-color": meta.color } as React.CSSProperties}>
            <Icon size={54} strokeWidth={1.25} />
            <span>{file.formatLabel}</span>
          </div>
        )}

        <div className="preview-topline">
          <span className="preview-family">
            <Icon size={12} />
            {meta.label}
          </span>
          <span className="preview-size">{humanSize(file.size)}</span>
        </div>

        <div className="preview-corner">
          {isFrame ? (
            <span className="preview-frame-badge">
              Frame <strong>{timecode(frameTime)}</strong>
            </span>
          ) : null}
          {video && file.duration ? (
            <span className="preview-duration">{timecode(file.duration)}</span>
          ) : null}
        </div>
      </div>

      <div className="preview-footer">
        <div className="preview-heading">
          <div className="preview-title-row">
            <FormatMark format={format} size="xs" />
            <h2 title={file.name}>{file.name}</h2>
          </div>
          <p>{fileDetail(file)}</p>
        </div>
        <div className="preview-actions">
          <button type="button" className="text-button" onClick={onReplace}>
            <RefreshCw size={12} />
            Replace
          </button>
          <button type="button" className="text-button is-danger" onClick={onRemove}>
            <Trash2 size={12} />
            Remove
          </button>
        </div>
      </div>
    </section>
  );
}
