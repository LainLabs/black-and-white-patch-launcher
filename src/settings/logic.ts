// Turning what the backend reads into form values and back, and the hints that explain how settings interact
// (rules from the decomp: src/core/Config.cpp, docs/RENDERER.md, docs/TEXTURES.md).

import type { IniValue, SettingsChange, SettingsSnapshot } from "@/lib/api";
import { FIELDS, type Values } from "./schema";

export const RESOLUTION_W = "game.ScreenW";
export const RESOLUTION_H = "game.ScreenH";

function iniLookup(values: IniValue[], section: string, key: string): string | undefined {
  const lower = (s: string) => s.toLowerCase();
  return values.find((v) => lower(v.section) === lower(section) && lower(v.key) === lower(key))?.value;
}

/** Form values from what is on disk; `defaults` = true gives the shipped defaults instead (registry untouched). */
export function toValues(snapshot: SettingsSnapshot, defaults = false): Values {
  const out: Values = {};
  for (const f of FIELDS) {
    if (f.store === "ini") {
      const source = defaults ? snapshot.defaults : snapshot.ini;
      out[f.id] = iniLookup(source, f.section, f.key) ?? iniLookup(snapshot.defaults, f.section, f.key) ?? f.fallback;
    } else {
      const v = snapshot.game[f.key];
      out[f.id] = v === undefined ? f.fallback : String(v);
    }
  }
  out[RESOLUTION_W] = snapshot.game.ScreenW !== undefined ? String(snapshot.game.ScreenW) : "";
  out[RESOLUTION_H] = snapshot.game.ScreenH !== undefined ? String(snapshot.game.ScreenH) : "";
  return out;
}

/** What changed between `before` and `after`, for write_settings. */
export function toChange(before: Values, after: Values): SettingsChange {
  const change: SettingsChange = { ini: [], game: {} };
  for (const f of FIELDS) {
    if (before[f.id] === after[f.id]) continue;
    if (f.store === "ini") {
      change.ini.push({ section: f.section, key: f.key, value: after[f.id] ?? "" });
    } else {
      const n = Number(after[f.id]);
      if (Number.isInteger(n) && n >= 0) change.game[f.key] = n;
    }
  }
  for (const id of [RESOLUTION_W, RESOLUTION_H]) {
    const n = Number(after[id]);
    if (after[id] !== before[id] && Number.isInteger(n) && n > 0) change.game[id.slice(5)] = n;
  }
  return change;
}

export function isEmptyChange(change: SettingsChange): boolean {
  return change.ini.length === 0 && Object.keys(change.game).length === 0;
}

/** Notes shown under fields whose effect depends on other settings. */
export function hints(v: Values): Record<string, string> {
  const out: Record<string, string> = {};
  const d3d9 = v["Display.Renderer"] !== "0";
  if (d3d9 && v["Display.WindowMode"] === "0") {
    out["Display.WindowMode"] =
      "The Direct3D 9 renderer never changes the display mode: full screen runs as a borderless window covering the screen.";
  }
  if (!d3d9) {
    for (const id of ["Display.AntiAliasing", "Display.Anisotropy", "Display.FullPrecisionTextures", "Textures.Replace", "Textures.Dump"]) {
      out[id] = "Only with the Direct3D 9 renderer (Display tab).";
    }
    const w = Number(v[RESOLUTION_W]);
    if (w > 2048) {
      out[RESOLUTION_W] = "The original renderer is at most 2048 pixels wide: the game uses the largest mode that works.";
    }
  } else if (v["Diagnostics.TraceDirectX"] === "1") {
    out["Diagnostics.TraceDirectX"] = "Traces the original renderer only; it has no effect with Direct3D 9.";
  }
  if (v["Fixes.DiscFromInstallFolder"] === "0") {
    out["Fixes.DiscFromInstallFolder"] = "The game will need the disc or a mounted image (drives D-Y) and quits after 30 seconds without one.";
  }
  return out;
}

// ---- Windows virtual-key codes for ScreenshotKey ----

const NAMED_KEYS: Record<number, string> = {
  19: "Pause", 33: "Page Up", 34: "Page Down", 35: "End", 36: "Home", 45: "Insert", 46: "Delete", 145: "Scroll Lock",
};

export function vkName(code: number): string {
  if (!code) return "None";
  if (code >= 112 && code <= 135) return `F${code - 111}`;
  if ((code >= 65 && code <= 90) || (code >= 48 && code <= 57)) return String.fromCharCode(code);
  return NAMED_KEYS[code] ?? `Key ${code}`;
}

/** The virtual-key code for a key press, or null for keys that cannot be used (Print Screen, modifiers). */
export function vkFromKey(key: string, code: string): number | null {
  const f = /^F(\d{1,2})$/.exec(key);
  if (f) {
    const n = Number(f[1]);
    return n >= 1 && n <= 24 ? 111 + n : null;
  }
  if (/^Key[A-Z]$/.test(code)) return code.charCodeAt(3);
  if (/^Digit[0-9]$/.test(code)) return code.charCodeAt(5);
  const named: Record<string, number> = {
    Pause: 19, PageUp: 33, PageDown: 34, End: 35, Home: 36, Insert: 45, Delete: 46, ScrollLock: 145,
  };
  return named[key] ?? null;
}
