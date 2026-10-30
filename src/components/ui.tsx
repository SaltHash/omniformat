import type { CSSProperties, ReactNode } from "react";
import { abbrev, familyMeta } from "../lib/families";
import type { FormatInfo } from "../lib/types";

export function Mark({ size = 18 }: { size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      aria-hidden="true"
      className="mark"
    >
      <path d="M3 9.5H13V4.5L21 12L13 19.5V14.5H3Z" fill="var(--accent)" />
    </svg>
  );
}

export function FormatMark({
  format,
  size = "md",
  tone = "default",
}: {
  format: FormatInfo | null;
  size?: "xs" | "sm" | "md" | "lg" | "xl";
  tone?: "default" | "plain";
}) {
  const meta = familyMeta(format?.family);
  const label = abbrev(format);
  const style = { "--mark-color": meta.color } as CSSProperties;
  return (
    <span
      className={`format-mark format-mark-${size} ${tone === "plain" ? "is-plain" : ""}`}
      style={style}
      aria-hidden="true"
    >
      <span>{label}</span>
    </span>
  );
}

export function Kbd({ children }: { children: ReactNode }) {
  return <kbd className="kbd">{children}</kbd>;
}

export function Spinner({ size = 16 }: { size?: number }) {
  return (
    <span className="spinner" style={{ width: size, height: size }} aria-hidden="true" />
  );
}

export function Switch({
  checked,
  onChange,
  label,
}: {
  checked: boolean;
  onChange: (value: boolean) => void;
  label?: string;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      className={`switch ${checked ? "is-on" : ""}`}
      onClick={() => onChange(!checked)}
    >
      <span className="switch-thumb" />
    </button>
  );
}

export interface SegmentOption<T extends string | number> {
  value: T;
  label: string;
}

export function Segmented<T extends string | number>({
  value,
  options,
  onChange,
  compact = false,
}: {
  value: T;
  options: SegmentOption<T>[];
  onChange: (value: T) => void;
  compact?: boolean;
}) {
  return (
    <div className={`segmented ${compact ? "is-compact" : ""}`} role="tablist">
      {options.map((option) => (
        <button
          key={String(option.value)}
          type="button"
          role="tab"
          aria-selected={option.value === value}
          className={option.value === value ? "is-active" : ""}
          onClick={() => onChange(option.value)}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}

export function Slider({
  value,
  min,
  max,
  step = 1,
  onChange,
  suffix,
  ariaLabel,
}: {
  value: number;
  min: number;
  max: number;
  step?: number;
  onChange: (value: number) => void;
  suffix?: string;
  ariaLabel: string;
}) {
  const percent = ((value - min) / Math.max(1, max - min)) * 100;
  return (
    <div className="slider-wrap">
      <input
        className="slider"
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        aria-label={ariaLabel}
        style={{ "--pct": `${percent}%` } as CSSProperties}
        onChange={(event) => onChange(Number(event.target.value))}
      />
      <span className="slider-value">
        {value}
        {suffix ? <span>{suffix}</span> : null}
      </span>
    </div>
  );
}

export function PageHeader({
  title,
  subtitle,
  actions,
  onTitleClick,
}: {
  title: string;
  subtitle?: string;
  actions?: ReactNode;
  onTitleClick?: () => void;
}) {
  return (
    <header className="page-header">
      <div className="page-heading">
        {onTitleClick ? (
          <h1 className="page-title">
            <button
              type="button"
              className="page-title-button"
              onClick={onTitleClick}
              title="Back to start"
            >
              {title}
            </button>
          </h1>
        ) : (
          <h1 className="page-title">{title}</h1>
        )}
        {subtitle ? <p className="page-sub">{subtitle}</p> : null}
      </div>
      {actions ? <div className="header-actions">{actions}</div> : null}
    </header>
  );
}

export function IconButton({
  children,
  onClick,
  label,
  variant = "default",
  disabled = false,
}: {
  children: ReactNode;
  onClick?: () => void;
  label: string;
  variant?: "default" | "danger" | "quiet";
  disabled?: boolean;
}) {
  return (
    <button
      type="button"
      className={`icon-button ${variant === "danger" ? "is-danger" : ""} ${variant === "quiet" ? "is-quiet" : ""}`}
      aria-label={label}
      title={label}
      onClick={onClick}
      disabled={disabled}
    >
      {children}
    </button>
  );
}

export function EmptyState({
  icon,
  title,
  description,
  action,
}: {
  icon: ReactNode;
  title: string;
  description: string;
  action?: ReactNode;
}) {
  return (
    <div className="empty-state">
      <div className="empty-icon">{icon}</div>
      <h3>{title}</h3>
      <p>{description}</p>
      {action ? <div className="empty-action">{action}</div> : null}
    </div>
  );
}
