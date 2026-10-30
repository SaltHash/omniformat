import {
  Check,
  CheckCircle2,
  ChevronDown,
  FolderOpen,
  Loader2,
  Square,
  X,
  XCircle,
} from "lucide-react";
import { useState } from "react";
import { formatById, humanSize } from "../lib/families";
import type { Catalog, JobView } from "../lib/types";
import { FormatMark, Spinner } from "./ui";

function percent(job: JobView): number {
  return Math.max(0, Math.min(100, job.progress || 0));
}

export function ActivityTray({
  jobs,
  catalog,
  onCancel,
  onClear,
  onReveal,
}: {
  jobs: JobView[];
  catalog: Catalog | null;
  onCancel: (id: string) => void;
  onClear: () => void;
  onReveal: (job: JobView) => void;
}) {
  const [expanded, setExpanded] = useState(false);
  if (!jobs.length) return null;

  const active = jobs.filter((job) => job.status === "queued" || job.status === "running");
  const finished = jobs.filter((job) => job.status !== "queued" && job.status !== "running");
  const running = jobs.find((job) => job.status === "running") ?? active[0];
  const progress =
    active.length > 0
      ? active.reduce((sum, job) => sum + percent(job), 0) / active.length
      : 100;
  const download = finished.reduce((sum, job) => sum + (job.sizeAfter ?? 0), 0);

  return (
    <div className={`activity-dock ${expanded ? "is-open" : ""}`}>
      {expanded ? (
        <div className="activity-panel">
          <div className="activity-panel-head">
            <div>
              <span className="eyebrow">Activity</span>
              <h3>
                {active.length
                  ? `${active.length} in progress`
                  : `${finished.length} finished`}
              </h3>
            </div>
            <div className="activity-panel-actions">
              {finished.length ? (
                <button type="button" className="text-button" onClick={onClear}>
                  Clear finished
                </button>
              ) : null}
              <button
                type="button"
                className="icon-button is-quiet"
                aria-label="Close activity"
                onClick={() => setExpanded(false)}
              >
                <X size={14} />
              </button>
            </div>
          </div>
          <div className="activity-list">
            {jobs.map((job) => {
              const target = formatById(catalog, job.target);
              const isRunning = job.status === "running" || job.status === "queued";
              const done = job.status === "done";
              const failed = job.status === "error";
              const cancelled = job.status === "cancelled";
              return (
                <div key={job.id} className={`activity-row is-${job.status}`}>
                  <FormatMark format={target} size="sm" />
                  <div className="activity-row-main">
                    <div className="activity-row-title">
                      <span title={job.inputName}>{job.inputName}</span>
                      <span className="activity-target">
                        <span>{job.mode}</span>
                        <span className="activity-arrow">→</span>
                        <strong>{job.targetLabel}</strong>
                      </span>
                    </div>
                    {isRunning ? (
                      <div className="activity-row-progress">
                        <div className="mini-track">
                          <span style={{ width: `${percent(job)}%` }} />
                        </div>
                        <span className="activity-stage">{job.stage}</span>
                        <span className="activity-percent">{Math.round(percent(job))}%</span>
                      </div>
                    ) : (
                      <div className="activity-row-result">
                        {done ? (
                          <>
                            <CheckCircle2 size={12} />
                            <span>{job.outputName}</span>
                            {job.sizeAfter ? (
                              <span className="activity-size">{humanSize(job.sizeAfter)}</span>
                            ) : null}
                          </>
                        ) : failed ? (
                          <>
                            <XCircle size={12} />
                            <span className="activity-error">{job.error ?? "Conversion failed"}</span>
                          </>
                        ) : cancelled ? (
                          <>
                            <XCircle size={12} />
                            <span>Cancelled</span>
                          </>
                        ) : (
                          <>
                            <Check size={12} />
                            <span>{job.stage}</span>
                          </>
                        )}
                      </div>
                    )}
                  </div>
                  <div className="activity-row-actions">
                    {isRunning ? (
                      <button
                        type="button"
                        className="row-action"
                        onClick={() => onCancel(job.id)}
                        title="Cancel"
                      >
                        <Square size={11} />
                      </button>
                    ) : null}
                    {done ? (
                      <button
                        type="button"
                        className="row-action"
                        onClick={() => onReveal(job)}
                        title="Show in folder"
                      >
                        <FolderOpen size={13} />
                      </button>
                    ) : null}
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      ) : null}

      <div className="activity-bar">
        <button
          type="button"
          className="activity-summary"
          onClick={() => setExpanded((value) => !value)}
          aria-expanded={expanded}
        >
          <span className={`activity-status ${active.length ? "is-busy" : "is-done"}`}>
            {active.length ? <Spinner size={14} /> : <CheckCircle2 size={15} />}
          </span>
          <span className="activity-summary-title">
            {active.length ? (
              <>
                <strong>Converting {active.length === 1 ? running?.inputName ?? "file" : `${active.length} files`}</strong>
                <span>
                  {running?.status === "running"
                    ? `${running.stage} · ${Math.round(percent(running))}%`
                    : "Queued"}
                </span>
              </>
            ) : (
              <>
                <strong>{finished.length} conversion{finished.length === 1 ? "" : "s"} finished</strong>
                <span>{download ? `${humanSize(download)} written` : "Ready when you are"}</span>
              </>
            )}
          </span>
          <span className="activity-overall">
            <span className="activity-overall-track">
              <span style={{ width: `${progress}%` }} />
            </span>
            <span>{active.length ? `${Math.round(progress)}%` : "Done"}</span>
          </span>
          <span className="activity-chevron">
            <ChevronDown size={14} />
          </span>
        </button>
        {active.length ? (
          <span className="activity-count">
            <Loader2 size={12} className="is-spinning" />
            {active.length}
          </span>
        ) : (
          <button type="button" className="activity-clear" onClick={onClear}>
            Clear
          </button>
        )}
      </div>
    </div>
  );
}
