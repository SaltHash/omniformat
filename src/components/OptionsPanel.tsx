import { Check, ChevronDown, Info, Sparkles } from "lucide-react";
import { useEffect, useState, type ReactNode } from "react";
import { timecode } from "../lib/families";
import type { FileMeta, FormatInfo, Options } from "../lib/types";
import { Segmented, Slider, Switch } from "./ui";

function Row({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: ReactNode;
}) {
  return (
    <div className="option-row">
      <div className="option-label">
        <span>{label}</span>
        {hint ? <small>{hint}</small> : null}
      </div>
      <div className="option-control">{children}</div>
    </div>
  );
}

function QualityRow({
  label,
  hint,
  value,
  onChange,
}: {
  label: string;
  hint?: string;
  value: number;
  onChange: (value: number) => void;
}) {
  return (
    <div className="option-quality">
      <div className="option-quality-head">
        <div className="option-label">
          <span>{label}</span>
          {hint ? <small>{hint}</small> : null}
        </div>
        <span className="quality-badge">{value}</span>
      </div>
      <Slider
        value={value}
        min={10}
        max={100}
        step={1}
        onChange={onChange}
        ariaLabel={label}
      />
    </div>
  );
}

function Note({
  title,
  description,
  icon = <Sparkles size={15} />,
}: {
  title: string;
  description: string;
  icon?: ReactNode;
}) {
  return (
    <div className="settings-note">
      <span className="settings-note-icon">{icon}</span>
      <div>
        <strong>{title}</strong>
        <span>{description}</span>
      </div>
    </div>
  );
}

const SEGMENT_CLASS = "is-compact";

export function OptionsPanel({
  file,
  target,
  options,
  onChange,
}: {
  file: FileMeta;
  target: FormatInfo;
  options: Options;
  onChange: (patch: Partial<Options>) => void;
}) {
  const [advanced, setAdvanced] = useState(false);
  useEffect(() => {
    setAdvanced(false);
  }, [file.path, target.id]);

  const source = file.family;
  const targetFamily = target.family;
  const quality = options.quality ?? 85;
  const maxDimension = options.maxDimension ?? 0;
  const videoHeight = options.videoHeight ?? 0;
  const fps = options.fps ?? 0;
  const isVideoFrame = source === "video" && targetFamily === "image" && target.id !== "gif";
  const isVideoGif = source === "video" && target.id === "gif";
  const isStill = (source === "image" || source === "vector") && targetFamily === "video";
  const isAudioVisual = source === "audio" && targetFamily === "video";
  const isPdfRaster = source === "pdf" && targetFamily === "image";
  const isPdfVector = source === "pdf" && (targetFamily === "vector" || ["svg", "ps", "eps"].includes(target.id));
  const isWaveform = source === "audio" && targetFamily === "image";
  const isImageTarget = targetFamily === "image" && !isPdfVector;
  const isLossyImage = isImageTarget && !target.lossless;
  const showQuality =
    isLossyImage ||
    targetFamily === "video" ||
    (targetFamily === "pdf" && (source === "image" || source === "vector"));

  const trimAvailable =
    (targetFamily === "video" || targetFamily === "audio") &&
    (source === "video" || source === "audio");
  const metadataAvailable = isImageTarget;
  const hasAdvanced = trimAvailable || metadataAvailable;

  return (
    <div className="options">
      {isVideoFrame ? (
        <Row
          label="Frame"
          hint={`of ${timecode(file.duration ?? 0)}`}
        >
          <div className="number-inline">
            <input
              className="number-input"
              type="number"
              min={0}
              max={Math.max(0, (file.duration ?? 0) - 0.05)}
              step={0.1}
              value={Number((options.frameTime ?? 0).toFixed(2))}
              onChange={(event) =>
                onChange({ frameTime: Math.max(0, Number(event.target.value)) })
              }
              aria-label="Frame time in seconds"
            />
            <span>sec</span>
          </div>
        </Row>
      ) : null}

      {isVideoGif ? (
        <>
          <Row label="Frame rate" hint="Smoother costs more">
            <Segmented
              compact
              value={options.fps ?? 12}
              options={[
                { value: 8, label: "8" },
                { value: 12, label: "12" },
                { value: 15, label: "15" },
                { value: 24, label: "24" },
              ]}
              onChange={(value) => onChange({ fps: value })}
            />
          </Row>
          <Row label="Width" hint="Pixels">
            <Segmented
              compact
              value={options.videoHeight ?? 480}
              options={[
                { value: 320, label: "320" },
                { value: 480, label: "480" },
                { value: 640, label: "640" },
                { value: 800, label: "800" },
              ]}
              onChange={(value) => onChange({ videoHeight: value })}
            />
          </Row>
        </>
      ) : null}

      {isStill ? (
        <div className="option-quality">
          <div className="option-quality-head">
            <div className="option-label">
              <span>Duration</span>
              <small>Each image becomes a clip</small>
            </div>
            <span className="quality-badge">{options.stillDuration ?? 5}s</span>
          </div>
          <Slider
            value={options.stillDuration ?? 5}
            min={1}
            max={20}
            step={1}
            onChange={(value) => onChange({ stillDuration: value })}
            ariaLabel="Clip duration"
          />
        </div>
      ) : null}

      {targetFamily === "video" ? (
        <>
          <Row label="Resolution" hint="Keeps aspect ratio">
            <Segmented
              compact
              value={videoHeight}
              options={[
                { value: 0, label: "Auto" },
                { value: 720, label: "720p" },
                { value: 1080, label: "1080p" },
                { value: 1440, label: "1440p" },
              ]}
              onChange={(value) => onChange({ videoHeight: value })}
            />
          </Row>
          {!isStill ? (
            <Row label="Frame rate" hint="Frames per second">
              <Segmented
                compact
                value={fps}
                options={[
                  { value: 0, label: "Auto" },
                  { value: 24, label: "24" },
                  { value: 30, label: "30" },
                  { value: 60, label: "60" },
                ]}
                onChange={(value) => onChange({ fps: value })}
              />
            </Row>
          ) : null}
        </>
      ) : null}

      {isImageTarget ? (
        <>
          <Row label="Max edge" hint="Original keeps full size">
            <Segmented
              compact
              value={maxDimension}
              options={[
                { value: 0, label: "Auto" },
                { value: 2048, label: "2K" },
                { value: 1280, label: "1280" },
                { value: 800, label: "800" },
              ]}
              onChange={(value) => onChange({ maxDimension: value })}
            />
          </Row>
          {isPdfRaster ? (
            <>
              <Row label="Page" hint={`${file.pages ?? 1} available`}>
                <div className="number-inline">
                  <input
                    className="number-input"
                    type="number"
                    min={1}
                    max={file.pages ?? undefined}
                    value={options.page ?? 1}
                    onChange={(event) =>
                      onChange({ page: Math.max(1, Number(event.target.value)) })
                    }
                    aria-label="PDF page number"
                  />
                  <span>of {file.pages ?? 1}</span>
                </div>
              </Row>
              <Row label="Density" hint="Raster resolution">
                <Segmented
                  compact
                  value={options.dpi ?? 150}
                  options={[
                    { value: 96, label: "96" },
                    { value: 150, label: "150" },
                    { value: 300, label: "300" },
                  ]}
                  onChange={(value) => onChange({ dpi: value })}
                />
              </Row>
            </>
          ) : null}
          {isWaveform ? (
            <Note
              title="Waveform preview"
              description="Rendered from the audio's amplitude envelope."
            />
          ) : null}
        </>
      ) : null}

      {isPdfVector ? (
        <Row label="Page" hint="Vector page selection">
          <div className="number-inline">
            <input
              className="number-input"
              type="number"
              min={1}
              max={file.pages ?? undefined}
              value={options.page ?? 1}
              onChange={(event) =>
                onChange({ page: Math.max(1, Number(event.target.value)) })
              }
              aria-label="PDF page number"
            />
            <span>of {file.pages ?? 1}</span>
          </div>
        </Row>
      ) : null}

      {targetFamily === "audio" ? (
        <>
          <Row label="Bitrate" hint="Higher is cleaner">
            <Segmented
              compact
              value={options.audioBitrate ?? 192}
              options={[
                { value: 128, label: "128" },
                { value: 192, label: "192" },
                { value: 256, label: "256" },
                { value: 320, label: "320" },
              ]}
              onChange={(value) => onChange({ audioBitrate: value })}
            />
          </Row>
          <Row label="Sample rate" hint="Hz">
            <Segmented
              compact
              value={options.sampleRate ?? 0}
              options={[
                { value: 0, label: "Auto" },
                { value: 44100, label: "44.1k" },
                { value: 48000, label: "48k" },
              ]}
              onChange={(value) => onChange({ sampleRate: value })}
            />
          </Row>
          <Row label="Channels">
            <Segmented
              compact
              value={options.channels ?? 0}
              options={[
                { value: 0, label: "Keep" },
                { value: 1, label: "Mono" },
                { value: 2, label: "Stereo" },
              ]}
              onChange={(value) => onChange({ channels: value })}
            />
          </Row>
          <Row label="Normalise" hint="Broadcast loudness">
            <Switch
              checked={options.normalizeAudio ?? false}
              onChange={(value) => onChange({ normalizeAudio: value })}
              label="Normalise audio"
            />
          </Row>
        </>
      ) : null}

      {showQuality ? (
        <QualityRow
          label="Quality"
          hint={targetFamily === "video" ? "Higher keeps more detail" : "Higher is larger"}
          value={quality}
          onChange={(value) => onChange({ quality: value })}
        />
      ) : null}

      {targetFamily === "image" && target.lossless && !isWaveform ? (
        <div className="lossless-line">
          <Check size={13} />
          Lossless output keeps every pixel
        </div>
      ) : null}

      {targetFamily === "archive" ? (
        <div className="option-quality">
          <div className="option-quality-head">
            <div className="option-label">
              <span>Compression</span>
              <small>Higher packs smaller</small>
            </div>
            <span className="quality-badge">{options.compression ?? 5}</span>
          </div>
          <Slider
            value={options.compression ?? 5}
            min={1}
            max={9}
            onChange={(value) => onChange({ compression: value })}
            ariaLabel="Archive compression"
          />
        </div>
      ) : null}

      {targetFamily === "pdf" ? (
        <Note
          title={source === "image" || source === "vector" ? "Pages follow selection order" : "Layout preserved"}
          description={
            source === "image" || source === "vector"
              ? "Each file becomes one page. Multi-file selections merge into a single PDF."
              : "Text, tables and page structure carry over from the source."
          }
        />
      ) : null}

      {["document", "spreadsheet", "presentation", "subtitle"].includes(targetFamily) ? (
        <Note
          title={targetFamily === "subtitle" ? "Timing preserved" : "Structure preserved"}
          description={
            targetFamily === "subtitle"
              ? "Cue timing and line breaks survive the format change."
              : "The document engine keeps layout, tables and embedded media."
          }
        />
      ) : null}

      {targetFamily === "video" && source !== "image" && source !== "vector" ? (
        <Row label="Audio" hint="Drop the soundtrack">
          <Switch
            checked={options.mute ?? false}
            onChange={(value) => onChange({ mute: value })}
            label="Remove audio"
          />
        </Row>
      ) : null}

      {hasAdvanced ? (
        <div className="advanced">
          <button
            type="button"
            className={`advanced-toggle ${advanced ? "is-open" : ""}`}
            onClick={() => setAdvanced((value) => !value)}
          >
            <span>Advanced</span>
            <ChevronDown size={14} />
          </button>
          {advanced ? (
            <div className="advanced-body">
              {isImageTarget ? (
                <Row label="Metadata" hint="Keep EXIF, ICC and profiles">
                  <Switch
                    checked={!(options.stripMetadata ?? false)}
                    onChange={(value) => onChange({ stripMetadata: !value })}
                    label="Keep metadata"
                  />
                </Row>
              ) : null}
              {trimAvailable ? (
                <div className="trim-block">
                  <div className="option-label">
                    <span>Trim</span>
                    <small>Seconds from the start</small>
                  </div>
                  <div className="trim-fields">
                    <label>
                      <span>Start</span>
                      <input
                        className="number-input"
                        type="number"
                        min={0}
                        step={0.1}
                        placeholder="0.0"
                        value={options.trimStart ?? ""}
                        onChange={(event) =>
                          onChange({
                            trimStart:
                              event.target.value === ""
                                ? null
                                : Math.max(0, Number(event.target.value)),
                          })
                        }
                      />
                    </label>
                    <span className="trim-dash">to</span>
                    <label>
                      <span>End</span>
                      <input
                        className="number-input"
                        type="number"
                        min={0}
                        step={0.1}
                        placeholder="Full"
                        value={options.trimEnd ?? ""}
                        onChange={(event) =>
                          onChange({
                            trimEnd:
                              event.target.value === ""
                                ? null
                                : Math.max(0, Number(event.target.value)),
                          })
                        }
                      />
                    </label>
                  </div>
                </div>
              ) : null}
              {!isAudioVisual && !isStill && targetFamily === "audio" ? (
                <div className="small-note">
                  <Info size={12} /> Normalising rewrites loudness for consistent playback.
                </div>
              ) : null}
            </div>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
