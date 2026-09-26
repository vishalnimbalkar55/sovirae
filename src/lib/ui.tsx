// Small shared controls so every screen uses the same states.
import type { CSSProperties, ReactNode } from "react";

export function Slider({ id, value, min, max, step, onChange, label, valueText }: {
  id?: string;
  value: number;
  min: number;
  max: number;
  step: number;
  onChange: (v: number) => void;
  label?: string;
  valueText?: string;
}) {
  const pct = ((value - min) / (max - min)) * 100;
  return (
    <input
      id={id}
      className="slider"
      type="range"
      min={min}
      max={max}
      step={step}
      value={value}
      aria-label={label}
      aria-valuetext={valueText}
      style={{ "--pct": `${pct}%` } as CSSProperties}
      onChange={(e) => onChange(Number(e.target.value))}
    />
  );
}

export function Switch({ id, checked, onChange, disabled }: {
  id: string;
  checked: boolean;
  onChange: (v: boolean) => void;
  disabled?: boolean;
}) {
  return (
    <input
      id={id}
      type="checkbox"
      role="switch"
      className="switch"
      checked={checked}
      disabled={disabled}
      onChange={(e) => onChange(e.target.checked)}
    />
  );
}

export function Segmented<T extends string>({ label, value, options, onChange }: {
  label: string;
  value: T;
  options: [T, string][];
  onChange: (v: T) => void;
}) {
  return (
    <div className="segmented" role="radiogroup" aria-label={label}>
      {options.map(([v, text]) => (
        <button key={v} role="radio" aria-checked={value === v} onClick={() => onChange(v)}>
          {text}
        </button>
      ))}
    </div>
  );
}

export function Row({ label, hint, children, htmlFor }: {
  label: string;
  hint?: ReactNode;
  children: ReactNode;
  htmlFor?: string;
}) {
  return (
    <div className="row">
      <div className="row-label">
        {htmlFor ? <label htmlFor={htmlFor}>{label}</label> : <span>{label}</span>}
        {hint && <span className="hint">{hint}</span>}
      </div>
      <div className="row-control">{children}</div>
    </div>
  );
}

export function Section({ title, description, children }: { title: string; description?: string; children: ReactNode }) {
  return (
    <section className="section">
      <div className="section-head">
        <h2>{title}</h2>
        {description && <p>{description}</p>}
      </div>
      {children}
    </section>
  );
}
