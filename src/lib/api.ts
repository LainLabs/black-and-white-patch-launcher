// Typed wrappers over the Rust commands in src-tauri/src/commands.rs. Keep the two in step.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type AppInfo = {
  launcherVersion: string;
  decompVersion: string;
  decompCommit: string;
  hasPayload: boolean;
  stateDir: string;
  addonsDir: string;
  elevated: boolean;
};

export type LauncherState = {
  gameFolder: string | null;
  onPlay: "keep" | "minimize" | "close";
  launcherRepo: string;
  patchRepo: string;
  checkUpdatesOnStart: boolean;
  gameArguments: string[];
};

export type GameBuild = "missing" | "unreadable" | "supported" | "retailSafeDisc" | "unknown";
export type GameCheck = { folder: string; build: GameBuild; sha256: string; message: string };

/** "newer": a patch from the patch's releases, newer than the one this launcher carries. */
export type PatchState = "notInstalled" | "installed" | "outdated" | "newer" | "byHand";
export type PatchStatus = {
  state: PatchState;
  installedVersion: string;
  bundledVersion: string;
  fromSetup: boolean;
  needsRights: boolean;
  running: boolean;
};

export type IniValue = { section: string; key: string; value: string };
export type SettingsSnapshot = {
  ini: IniValue[];
  iniExists: boolean;
  defaults: IniValue[];
  game: Record<string, number>;
};
export type SettingsChange = { ini: IniValue[]; game: Record<string, number> };

export type DisplayMode = { width: number; height: number };
export type DisplayModes = { modes: DisplayMode[]; current: DisplayMode | null };

export type GameEvent = { state: "started" | "failed" | "exited"; launcherExitCode: number | null; logTail: string };

export type AddonManifest = {
  id: string;
  name: string;
  version: string;
  author: string;
  description: string;
  kind: string;
  homepage: string;
  requires: { decomp?: string | null };
  settings: IniValue[];
};
export type AddonInfo = { manifest: AddonManifest; path: string; enabled: boolean; problem: string; fileCount: number };

export type Asset = { name: string; url: string; size: number };
export type Product = "launcher" | "patch";
export type UpdateInfo = {
  product: Product;
  currentVersion: string;
  latestVersion: string;
  newer: boolean;
  name: string;
  notes: string;
  pageUrl: string;
  publishedAt: string;
  /** The setup program (launcher) or zip (patch). */
  download: Asset | null;
  sums: Asset | null;
};

export const api = {
  appInfo: () => invoke<AppInfo>("app_info"),
  getState: () => invoke<LauncherState>("get_state"),
  setState: (value: LauncherState) => invoke<void>("set_state", { value }),
  detectGameFolder: () => invoke<string | null>("detect_game_folder"),
  checkGame: (folder: string) => invoke<GameCheck>("check_game", { folder }),
  patchStatus: (folder: string) => invoke<PatchStatus>("patch_status", { folder }),
  installPatch: (folder: string) => invoke<void>("install_patch", { folder }),
  removePatch: (folder: string) => invoke<{ kept: string[] }>("remove_patch", { folder }),
  readSettings: (folder: string) => invoke<SettingsSnapshot>("read_settings", { folder }),
  writeSettings: (folder: string, change: SettingsChange) => invoke<void>("write_settings", { folder, change }),
  displayModes: () => invoke<DisplayModes>("display_modes"),
  launchGame: (folder: string) => invoke<void>("launch_game", { folder }),
  gameRunning: (folder: string) => invoke<boolean>("game_running", { folder }),
  readLogTail: (folder: string, lines: number) => invoke<string>("read_log_tail", { folder, lines }),
  openFolder: (path: string, create = false) => invoke<void>("open_folder", { path, create }),
  listAddons: (folder: string | null) => invoke<AddonInfo[]>("list_addons", { folder }),
  importAddon: (source: string) => invoke<string>("import_addon", { source }),
  deleteAddon: (id: string) => invoke<void>("delete_addon", { id }),
  setAddonEnabled: (folder: string, id: string, enabled: boolean) =>
    invoke<void>("set_addon_enabled", { folder, id, enabled }),
  /** A patch is compared with what `folder` has installed, else with the bundled patch. */
  checkUpdates: (product: Product, repo: string, folder: string | null) =>
    invoke<UpdateInfo>("check_updates", { product, repo, folder }),
  installLauncherUpdate: (info: UpdateInfo) => invoke<void>("install_launcher_update", { info }),
  installPatchUpdate: (folder: string, info: UpdateInfo) => invoke<void>("install_patch_update", { folder, info }),
  onGameEvent: (handler: (e: GameEvent) => void): Promise<UnlistenFn> =>
    listen<GameEvent>("game", (event) => handler(event.payload)),
};

/** Error text from a rejected command (the backend sends strings). */
export function errorText(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return JSON.stringify(error);
}

/** Joins a Windows path. */
export function joinPath(folder: string, ...parts: string[]): string {
  return [folder.replace(/[\\/]+$/, ""), ...parts].join("\\");
}
