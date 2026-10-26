// Development only: when the UI runs in a plain browser (`npm run dev`, no Tauri), answer the backend's commands
// with fake data so every page can be previewed and screenshotted. Never part of a release build (main.tsx only
// imports this when import.meta.env.DEV and there is no Tauri runtime).
//
// URL options: ?mock=fresh shows the first-run wizard; default is an installed game.

import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type { AddonInfo, LauncherState, SettingsSnapshot } from "./api";

export function installDevMock() {
  const fresh = new URLSearchParams(location.search).get("mock") === "fresh";
  const folder = "C:\\Games\\Black & White";
  let state: LauncherState = {
    gameFolder: fresh ? null : folder,
    onPlay: "minimize",
    updateRepo: "example/BlackWhiteLauncher",
    checkUpdatesOnStart: true,
    gameArguments: [],
  };
  const ini = [
    ["Fixes", "DirectXVersionReport", "1"], ["Fixes", "DiscFromInstallFolder", "1"], ["Fixes", "ColorDepth32", "1"],
    ["Fixes", "DisplayModeFallback", "1"], ["Fixes", "CloseRequestsQuit", "1"], ["Fixes", "MinimisedWait", "1"],
    ["Fixes", "ShutdownTimeoutSeconds", "60"], ["Fixes", "DpiAware", "1"], ["Fixes", "ExecutableSelfModifyingCode", "1"],
    ["Fixes", "MaxReportedPhysicalMemoryMB", "512"], ["Fixes", "MaxReportedVideoMemoryMB", "1024"],
    ["Display", "WindowMode", "0"], ["Display", "KeepAspectRatio", "1"], ["Display", "Renderer", "1"],
    ["Display", "VSync", "1"], ["Display", "Anisotropy", "0"], ["Display", "AntiAliasing", "0"],
    ["Display", "FullPrecisionTextures", "1"], ["Textures", "Dump", "0"], ["Textures", "Replace", "1"],
    ["Hooks", "Disabled", ""], ["Diagnostics", "TraceExit", "1"], ["Diagnostics", "TraceExceptions", "1"],
    ["Diagnostics", "ForwardDebugOutput", "0"], ["Diagnostics", "TraceDirectX", "0"], ["Diagnostics", "ScreenshotKey", "0"],
    ["Log", "Debug", "0"],
  ].map(([section, key, value]) => ({ section, key, value }));
  const settings: SettingsSnapshot = {
    ini: structuredClone(ini),
    iniExists: true,
    defaults: ini,
    game: { ScreenW: 2560, ScreenH: 1440, detailidx: 4, maxdetailidx: 5, AudioMusicMasterVolume: 127, AudioSampleMasterVolume: 100 },
  };
  const addons: AddonInfo[] = [
    {
      manifest: {
        id: "hd-skies", name: "HD Skies", version: "1.2.0", author: "Example author", kind: "texture-pack", homepage: "",
        description: "Sharper sky and cloud textures for the Direct3D 9 renderer.", requires: { decomp: ">=0.10.0" }, settings: [],
      },
      path: "C:\\Users\\you\\AppData\\Roaming\\BWLauncher\\addons\\hd-skies", enabled: true, problem: "", fileCount: 42,
    },
    {
      manifest: {
        id: "crisp", name: "Crisp preset", version: "1.0.0", author: "", kind: "preset", homepage: "",
        description: "8× anti-aliasing and 16× anisotropic filtering.", requires: {}, settings: [],
      },
      path: "C:\\Users\\you\\AppData\\Roaming\\BWLauncher\\addons\\crisp", enabled: false, problem: "", fileCount: 2,
    },
  ];

  mockWindows("main");
  mockIPC((cmd, args) => {
    const a = (args ?? {}) as Record<string, unknown>;
    switch (cmd) {
      case "app_info":
        return { launcherVersion: "0.1.0", decompVersion: "0.11.0", decompCommit: "938ed3d", hasPayload: true, stateDir: "%APPDATA%\\BWLauncher", addonsDir: "%APPDATA%\\BWLauncher\\addons", elevated: false };
      case "get_state":
        return state;
      case "set_state":
        state = a.value as LauncherState;
        return null;
      case "detect_game_folder":
        return folder;
      case "check_game":
        return { folder: a.folder, build: "supported", sha256: "86a30b08ab2ea8a240403d37f8f0303cf65aa5cde8e7f673049ab01654ccf755", message: "" };
      case "patch_status":
        return { state: fresh ? "notInstalled" : "installed", installedVersion: fresh ? "" : "0.11.0", bundledVersion: "0.11.0", fromSetup: false, needsRights: false, running: false };
      case "install_patch":
        state = { ...state, gameFolder: a.folder as string };
        return null;
      case "read_settings":
        return settings;
      case "write_settings":
        return null;
      case "display_modes":
        return { modes: [[2560, 1440], [1920, 1080], [1600, 900], [1280, 720], [1024, 768], [800, 600]].map(([width, height]) => ({ width, height })), current: { width: 2560, height: 1440 } };
      case "list_addons":
        return addons;
      case "check_updates":
        return { currentVersion: "0.1.0", latestVersion: "0.2.0", newer: true, name: "Launcher 0.2.0", notes: "- Decompilation 0.12.0\n- Faster texture pack switching", pageUrl: "https://github.com/example", publishedAt: "", installer: null, sums: null };
      case "read_log_tail":
        return "[00:00:01] BWDecomp 0.11.0 attached to runblack.exe\n[00:00:01] Installed 24 hooks";
      case "plugin:event|listen":
        return 1;
      default:
        return null;
    }
  });
}
