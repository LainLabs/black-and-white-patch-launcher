// One row of the settings form, rendered from its schema entry.

import { useEffect, useState } from "react";
import { Keyboard } from "lucide-react";
import type { Field, Values } from "@/settings/schema";
import { RESOLUTION_H, RESOLUTION_W, vkFromKey, vkName } from "@/settings/logic";
import { useDisplayModes } from "@/lib/queries";
import { Button, Segmented, Switch, cx, inputClass } from "./ui";

type RowProps = { label: string; help?: string; hint?: string; children: React.ReactNode; htmlFor?: string };

export function Row({ label, help, hint, children, htmlFor }: RowProps) {
  return (
    <div className="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-x-6 gap-y-1 border-b border-ink-800 py-4 last:border-b-0">
      <div className="min-w-0">
        <label htmlFor={htmlFor} className="text-sm font-medium text-ivory-50">
          {label}
        </label>
        {help && <p className="mt-0.5 text-xs leading-relaxed text-ivory-400">{help}</p>}
        {hint && <p className="mt-1 text-xs text-gold-300">{hint}</p>}
      </div>
      <div className="flex justify-end">{children}</div>
    </div>
  );
}

export function SettingField({
  field,
  values,
  onChange,
  hint,
  maxDetail,
}: {
  field: Field;
  values: Values;
  onChange: (id: string, value: string) => void;
  hint?: string;
  maxDetail?: number;
}) {
  const value = values[field.id] ?? field.fallback;
  const set = (v: string) => onChange(field.id, v);
  let control: React.ReactNode;
  switch (field.type) {
    case "bool":
      control = <Switch label={field.label} checked={value === "1"} onChange={(on) => set(on ? "1" : "0")} />;
      break;
    case "choice":
      control =
        (field.options?.length ?? 0) <= 5 ? (
          <Segmented label={field.label} value={value} options={field.options ?? []} onChange={set} />
        ) : (
          <select id={field.id} className={inputClass} value={value} onChange={(e) => set(e.target.value)}>
            {field.options?.map((o) => (
              <option key={o.value} value={o.value}>
                {o.label}
              </option>
            ))}
          </select>
        );
      break;
    case "int": {
      const max = field.id === "game.detailidx" && maxDetail !== undefined ? maxDetail : (field.max ?? 100);
      control = (
        <div className="flex items-center gap-3">
          <input
            id={field.id}
            type="range"
            min={field.min ?? 0}
            max={max}
            step={field.step ?? 1}
            value={Number(value) || 0}
            onChange={(e) => set(e.target.value)}
            className="w-40"
          />
          <input
            aria-label={`${field.label} value`}
            className={cx(inputClass, "w-20 text-right")}
            inputMode="numeric"
            value={value}
            onChange={(e) => set(e.target.value.replace(/[^0-9]/g, ""))}
          />
          {field.unit && <span className="w-6 text-xs text-ivory-400">{field.unit}</span>}
        </div>
      );
      break;
    }
    case "vkey":
      control = <KeyCapture value={Number(value) || 0} onChange={(n) => set(String(n))} />;
      break;
    case "text":
      control = (
        <input
          id={field.id}
          className={cx(inputClass, "w-80 font-mono")}
          value={value}
          spellCheck={false}
          onChange={(e) => set(e.target.value)}
          placeholder="(none)"
        />
      );
      break;
  }
  return (
    <Row label={field.label} help={field.help} hint={hint} htmlFor={field.id}>
      {control}
    </Row>
  );
}

function KeyCapture({ value, onChange }: { value: number; onChange: (code: number) => void }) {
  const [listening, setListening] = useState(false);
  useEffect(() => {
    if (!listening) return;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      if (e.key === "Escape") return setListening(false);
      const code = vkFromKey(e.key, e.code);
      if (code !== null) {
        onChange(code);
        setListening(false);
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [listening, onChange]);
  return (
    <div className="flex items-center gap-2">
      <Button
        size="sm"
        icon={<Keyboard className="size-4" />}
        onClick={() => setListening((l) => !l)}
        className={listening ? "border-gold-400 text-gold-300" : undefined}
      >
        {listening ? "Press a key… (Esc cancels)" : vkName(value)}
      </Button>
      {value !== 0 && !listening && (
        <Button size="sm" variant="ghost" onClick={() => onChange(0)}>
          Clear
        </Button>
      )}
    </div>
  );
}

/** ScreenW × ScreenH, from the display's modes or typed in. */
export function ResolutionField({
  values,
  onChange,
  hint,
}: {
  values: Values;
  onChange: (id: string, v: string) => void;
  hint?: string;
}) {
  const modes = useDisplayModes();
  const w = values[RESOLUTION_W] ?? "";
  const h = values[RESOLUTION_H] ?? "";
  const current = w && h ? `${w}x${h}` : "";
  const list = modes.data?.modes ?? [];
  const known = list.some((m) => `${m.width}x${m.height}` === current);
  const native = modes.data?.current;
  return (
    <Row
      label="Resolution"
      help={`The game renders at this size${native ? `; your display is ${native.width}×${native.height}` : ""}. Without a value the game uses 800×600.`}
      hint={hint}
      htmlFor="resolution"
    >
      <div className="flex items-center gap-2">
        <select
          id="resolution"
          className={cx(inputClass, "w-44")}
          value={known ? current : "custom"}
          onChange={(e) => {
            if (e.target.value === "custom") return;
            const [nw, nh] = e.target.value.split("x");
            onChange(RESOLUTION_W, nw);
            onChange(RESOLUTION_H, nh);
          }}
        >
          {list.map((m) => (
            <option key={`${m.width}x${m.height}`} value={`${m.width}x${m.height}`}>
              {m.width} × {m.height}
              {native && native.width === m.width && native.height === m.height ? " (native)" : ""}
            </option>
          ))}
          <option value="custom">{current && !known ? `${w} × ${h} (custom)` : "Custom…"}</option>
        </select>
        <input
          aria-label="Width"
          className={cx(inputClass, "w-20 text-right")}
          value={w}
          inputMode="numeric"
          onChange={(e) => onChange(RESOLUTION_W, e.target.value.replace(/[^0-9]/g, ""))}
        />
        <span className="text-ivory-400">×</span>
        <input
          aria-label="Height"
          className={cx(inputClass, "w-20 text-right")}
          value={h}
          inputMode="numeric"
          onChange={(e) => onChange(RESOLUTION_H, e.target.value.replace(/[^0-9]/g, ""))}
        />
      </div>
    </Row>
  );
}
