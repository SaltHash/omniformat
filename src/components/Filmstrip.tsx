import { Film } from "lucide-react";
import { timecode } from "../lib/families";
import type { FilmFrame } from "../lib/types";
import { Spinner, Slider } from "./ui";

export function Filmstrip({
  frames,
  duration,
  time,
  loading,
  onChange,
}: {
  frames: FilmFrame[];
  duration: number;
  time: number;
  loading: boolean;
  onChange: (time: number) => void;
}) {
  const nearest = frames.reduce(
    (best, frame, index) =>
      Math.abs(frame.time - time) < Math.abs((frames[best]?.time ?? 0) - time)
        ? index
        : best,
    0,
  );

  return (
    <section className="filmstrip" aria-label="Frame picker">
      <div className="filmstrip-head">
        <div className="filmstrip-title">
          <span className="filmstrip-icon">
            <Film size={13} />
          </span>
          <span>Pick a frame</span>
          {loading ? <Spinner size={12} /> : null}
        </div>
        <div className="filmstrip-time">
          <strong>{timecode(time)}</strong>
          <span>/ {timecode(duration)}</span>
        </div>
      </div>

      <div className="filmstrip-frames">
        {frames.length
          ? frames.map((frame, index) => (
              <button
                key={`${frame.time}-${index}`}
                type="button"
                className={`film-frame ${index === nearest ? "is-active" : ""}`}
                onClick={() => onChange(frame.time)}
                aria-label={`Use frame at ${timecode(frame.time)}`}
              >
                <img src={frame.image} alt="" draggable={false} />
                <span>{timecode(frame.time)}</span>
              </button>
            ))
          : Array.from({ length: 8 }, (_, index) => (
              <div key={index} className="film-frame is-skeleton" />
            ))}
      </div>

      <div className="filmstrip-scrub">
        <Slider
          value={Number(time.toFixed(2))}
          min={0}
          max={Math.max(0.1, duration)}
          step={0.05}
          onChange={(value) => onChange(value)}
          ariaLabel="Frame position"
        />
      </div>
    </section>
  );
}
