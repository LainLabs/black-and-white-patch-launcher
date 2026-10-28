// Small UI kit: buttons, switches, fields, cards and notices in the launcher's style.

import type { ButtonHTMLAttributes, ReactNode } from "react";
import { AlertTriangle, CheckCircle2, Info, Loader2, XCircle } from "lucide-react";

export function cx(...parts: (string | false | null | undefined)[]): string {
  return parts.filter(Boolean).join(" ");
}

type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: "primary" | "secondary" | "ghost" | "danger";
  size?: "sm" | "md" | "lg";
  busy?: boolean;
  icon?: ReactNode;
};

export function Button({
  variant = "secondary",
  size = "md",
  busy,
  icon,
  className,
  children,
  disabled,
  ...rest
}: ButtonProps) {
  return (
    <button
      {...rest}
      disabled={disabled || busy}
      className={cx(
        "inline-flex shrink-0 items-center justify-center gap-2 whitespace-nowrap rounded-lg font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-50",
        size === "sm" && "h-8 px-3 text-sm",
        size === "md" && "h-10 px-4 text-sm",
        size === "lg" && "h-14 px-8 text-lg",
        variant === "primary" &&
          "bg-gold-400 text-ink-950 hover:bg-gold-300 shadow-[0_0_24px_-6px_var(--color-gold-500)]",
        variant === "secondary" && "border border-ink-600 bg-ink-800 text-ivory-100 hover:bg-ink-700",
        variant === "ghost" && "text-ivory-300 hover:bg-ink-800 hover:text-ivory-50",
        variant === "danger" && "border border-evil-400/50 bg-evil-400/10 text-evil-400 hover:bg-evil-400/20",
        className,
      )}
    >
      {busy ? <Loader2 className="size-4 animate-spin" /> : icon}
      {children}
    </button>
  );
}

export function Switch({
  checked,
  onChange,
  disabled,
  label,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
  disabled?: boolean;
  label: string;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={cx(
        "relative inline-flex h-6 w-11 shrink-0 items-center rounded-full border transition-colors disabled:opacity-50",
        checked ? "border-gold-500 bg-gold-500/80" : "border-ink-600 bg-ink-700",
      )}
    >
      <span
        className={cx(
          "inline-block size-4 rounded-full bg-ivory-50 shadow transition-transform",
          checked ? "translate-x-6" : "translate-x-1",
        )}
      />
    </button>
  );
}

export function Segmented({
  value,
  options,
  onChange,
  label,
}: {
  value: string;
  options: { value: string; label: string }[];
  onChange: (v: string) => void;
  label: string;
}) {
  return (
    <div
      role="radiogroup"
      aria-label={label}
      className="inline-flex flex-wrap rounded-lg border border-ink-600 bg-ink-850 p-0.5"
    >
      {options.map((o) => (
        <button
          key={o.value}
          type="button"
          role="radio"
          aria-checked={value === o.value}
          onClick={() => onChange(o.value)}
          className={cx(
            "rounded-md px-3 py-1.5 text-sm transition-colors",
            value === o.value ? "bg-ink-600 text-ivory-50" : "text-ivory-400 hover:text-ivory-100",
          )}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}

export const inputClass =
  "h-10 rounded-lg border border-ink-600 bg-ink-850 px-3 text-sm text-ivory-50 placeholder:text-ink-500 focus:border-gold-500 focus:outline-none";

export function Card({
  title,
  actions,
  children,
  className,
}: {
  title?: ReactNode;
  actions?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <section className={cx("rounded-xl border border-ink-700 bg-ink-900/80 p-5", className)}>
      {(title || actions) && (
        <header className="mb-4 flex items-center justify-between gap-4">
          {title && <h2 className="font-display text-lg text-ivory-50">{title}</h2>}
          {actions}
        </header>
      )}
      {children}
    </section>
  );
}

const noticeStyles = {
  info: { icon: Info, cls: "border-ink-600 bg-ink-850 text-ivory-300" },
  success: { icon: CheckCircle2, cls: "border-good-400/40 bg-good-400/10 text-good-400" },
  warning: { icon: AlertTriangle, cls: "border-gold-500/40 bg-gold-500/10 text-gold-300" },
  error: { icon: XCircle, cls: "border-evil-400/40 bg-evil-400/10 text-evil-400" },
};

export function Notice({
  kind = "info",
  title,
  children,
}: {
  kind?: keyof typeof noticeStyles;
  title?: ReactNode;
  children?: ReactNode;
}) {
  const { icon: Icon, cls } = noticeStyles[kind];
  return (
    <div className={cx("flex gap-3 rounded-lg border p-3 text-sm", cls)} role={kind === "error" ? "alert" : "status"}>
      <Icon className="mt-0.5 size-4 shrink-0" />
      <div className="min-w-0 space-y-1">
        {title && <div className="font-medium">{title}</div>}
        {children && <div className="whitespace-pre-line text-ivory-300">{children}</div>}
      </div>
    </div>
  );
}

export function Badge({
  tone = "neutral",
  children,
}: {
  tone?: "neutral" | "good" | "warn" | "bad" | "gold";
  children: ReactNode;
}) {
  return (
    <span
      className={cx(
        "inline-flex items-center gap-1 rounded-full border px-2.5 py-0.5 text-xs font-medium",
        tone === "neutral" && "border-ink-600 text-ivory-400",
        tone === "good" && "border-good-400/40 text-good-400",
        tone === "warn" && "border-gold-500/40 text-gold-300",
        tone === "bad" && "border-evil-400/40 text-evil-400",
        tone === "gold" && "border-gold-400/60 bg-gold-400/10 text-gold-300",
      )}
    >
      {children}
    </span>
  );
}

/** The launcher's mark: a light and dark half circle. */
export function Mark({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 64 64" className={className} aria-hidden>
      <circle cx="32" cy="32" r="30" fill="var(--color-gold-500)" />
      <circle cx="32" cy="32" r="27" fill="var(--color-ivory-100)" />
      <path d="M32 5a27 27 0 0 0 0 54z" fill="var(--color-ink-900)" />
      <path d="M32 14c-7 7-7 29 0 36c7-7 7-29 0-36z" fill="var(--color-ink-900)" />
      <path d="M32 14c-7 7-7 29 0 36" fill="var(--color-ivory-100)" />
    </svg>
  );
}
