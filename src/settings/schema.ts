// Every setting the launcher edits, in one table. The settings page, the first-run quick setup and the tests are
// generated from it.
//
// INI settings: BWDecomp.ini, read by BWDecomp.dll. Source of truth in the decomp repo: src/core/Config.h (struct and
// defaults), src/core/Config.cpp (parsing, clamping) and the comments in BWDecomp.ini. Adding a key there means adding
// one entry here; schema.test.ts fails while the shipped BWDecomp.ini has a key this table lacks.
//
// Game settings: the game's own DWORD values in HKCU\Software\Lionhead Studios Ltd\Black & White\BWSetup.

export type Tab = "game" | "display" | "graphics" | "textures" | "fixes" | "diagnostics" | "advanced";

export const TABS: { id: Tab; label: string; blurb: string }[] = [
  { id: "game", label: "Game", blurb: "Resolution, detail and volume: the game's own settings." },
  { id: "display", label: "Display", blurb: "Full screen, window or borderless, and the renderer." },
  { id: "graphics", label: "Graphics", blurb: "Anti-aliasing, texture filtering and colour precision (Direct3D 9 renderer)." },
  { id: "textures", label: "Textures", blurb: "Texture packs: dump the game's textures and draw replacements." },
  { id: "fixes", label: "Fixes", blurb: "Compatibility fixes for current Windows. Leave them on unless you are testing." },
  { id: "diagnostics", label: "Diagnostics", blurb: "Logging and screenshots." },
  { id: "advanced", label: "Advanced", blurb: "For developers bisecting a problem." },
];

export type Option = { value: string; label: string };

export type Field = {
  /** "Section.Key" for INI settings, "game.<Value>" for registry values. */
  id: string;
  store: "ini" | "game";
  section: string;
  key: string;
  tab: Tab;
  label: string;
  help: string;
  type: "bool" | "int" | "choice" | "vkey" | "text";
  options?: Option[];
  min?: number;
  max?: number;
  step?: number;
  unit?: string;
  /** Shown on the first-run quick setup page. */
  quick?: boolean;
  /** Fallback when neither the file nor the shipped defaults have a value. */
  fallback: string;
};

const ini = (f: Omit<Field, "id" | "store">): Field => ({ ...f, id: `${f.section}.${f.key}`, store: "ini" });
const game = (f: Omit<Field, "id" | "store" | "section">): Field => ({
  ...f,
  id: `game.${f.key}`,
  store: "game",
  section: "BWSetup",
});

const samples = (labelOff: string): Option[] => [
  { value: "0", label: labelOff },
  { value: "2", label: "2×" },
  { value: "4", label: "4×" },
  { value: "8", label: "8×" },
  { value: "16", label: "16×" },
];

export const FIELDS: Field[] = [
  // ---- Game (registry) ----
  game({
    key: "ScreenD", tab: "game", label: "Colour depth", type: "choice", fallback: "32",
    options: [{ value: "32", label: "32-bit" }, { value: "16", label: "16-bit" }],
    help: "The game defaults to 16-bit; with the ColorDepth32 fix it renders at 32 bits either way.",
  }),
  game({
    key: "detailidx", tab: "game", label: "Detail level", type: "int", min: 0, max: 5, step: 1, fallback: "4",
    help: "The game's detail setting (0 = lowest). The highest level is the game's maxdetailidx.",
  }),
  game({
    key: "AudioMusicMasterVolume", tab: "game", label: "Music volume", type: "int", min: 0, max: 127, step: 1,
    fallback: "127", help: "0-127, as the game stores it.",
  }),
  game({
    key: "AudioSampleMasterVolume", tab: "game", label: "Sound volume", type: "int", min: 0, max: 127, step: 1,
    fallback: "127", help: "0-127, as the game stores it.",
  }),

  // ---- Display ----
  ini({
    section: "Display", key: "WindowMode", tab: "display", label: "Window mode", type: "choice", fallback: "0",
    quick: true,
    options: [
      { value: "0", label: "Full screen" },
      { value: "1", label: "Window" },
      { value: "2", label: "Borderless window" },
    ],
    help: "In a window or borderless, the picture is scaled to the window and Alt+Tab is instant.",
  }),
  ini({
    section: "Display", key: "KeepAspectRatio", tab: "display", label: "Keep aspect ratio", type: "bool", fallback: "1",
    help: "When the picture is scaled to the window, add black bars instead of stretching it.",
  }),
  ini({
    section: "Display", key: "Renderer", tab: "display", label: "Renderer", type: "choice", fallback: "1",
    options: [
      { value: "1", label: "Direct3D 9 (recommended)" },
      { value: "0", label: "Original (DirectDraw / Direct3D 7)" },
    ],
    help: "Direct3D 9 renders any resolution (e.g. 2560×1440) without display-mode changes and enables the graphics options. The original renderer is at most 2048 pixels wide.",
  }),
  ini({
    section: "Display", key: "VSync", tab: "display", label: "Vertical sync", type: "bool", fallback: "1", quick: true,
    help: "Wait for the display's refresh: no tearing.",
  }),

  // ---- Graphics ----
  ini({
    section: "Display", key: "AntiAliasing", tab: "graphics", label: "Anti-aliasing", type: "choice", fallback: "0",
    quick: true, options: samples("Off"), help: "Multisampling: smoother edges. 4× or 8× suits most cards.",
  }),
  ini({
    section: "Display", key: "Anisotropy", tab: "graphics", label: "Anisotropic filtering", type: "choice",
    fallback: "0", quick: true, options: samples("Off"), help: "Sharper ground in the distance.",
  }),
  ini({
    section: "Display", key: "FullPrecisionTextures", tab: "graphics", label: "Full-precision textures", type: "bool",
    fallback: "1",
    help: "Compressed textures the game unpacks into 16-bit textures keep their full colours.",
  }),

  // ---- Textures ----
  ini({
    section: "Textures", key: "Replace", tab: "textures", label: "Use texture packs", type: "bool", fallback: "1",
    help: "Draw images from Textures\\Replace\\ (and folders below it) instead of the game's textures: any size, 32-bit with alpha, PNG, DDS, BMP, JPEG or TIFF.",
  }),
  ini({
    section: "Textures", key: "Dump", tab: "textures", label: "Dump textures", type: "bool", fallback: "0",
    help: "Save every texture the game draws, once, to Textures\\Dump\\, for making replacements.",
  }),

  // ---- Fixes ----
  ini({
    section: "Fixes", key: "DirectXVersionReport", tab: "fixes", label: "Report DirectX 9.0c", type: "bool",
    fallback: "1", help: "Without it the game quits with \"You need DirectX 7 or greater\".",
  }),
  ini({
    section: "Fixes", key: "DiscFromInstallFolder", tab: "fixes", label: "Play without the disc", type: "bool",
    fallback: "1", help: "When no disc or image labelled BNW is found, use the install folder. Off = the original disc check.",
  }),
  ini({
    section: "Fixes", key: "ColorDepth32", tab: "fixes", label: "Run 16-bit modes at 32 bits", type: "bool",
    fallback: "1", help: "Current drivers cannot create a Direct3D 7 device on a 16-bit mode; the game would crash.",
  }),
  ini({
    section: "Fixes", key: "DisplayModeFallback", tab: "fixes", label: "Display mode fallback", type: "bool",
    fallback: "1", help: "If a resolution cannot be rendered, use the largest smaller one with the same aspect ratio instead of crashing.",
  }),
  ini({
    section: "Fixes", key: "CloseRequestsQuit", tab: "fixes", label: "Close requests quit", type: "bool", fallback: "1",
    help: "Closing the window quits the game like Alt+F4 instead of leaving it running without a window.",
  }),
  ini({
    section: "Fixes", key: "MinimisedWait", tab: "fixes", label: "Idle while minimised", type: "bool", fallback: "1",
    help: "Don't keep a CPU core busy while minimised, and allow quitting from the taskbar.",
  }),
  ini({
    section: "Fixes", key: "ShutdownTimeoutSeconds", tab: "fixes", label: "Shutdown timeout", type: "int", min: 0,
    max: 600, step: 5, unit: "s", fallback: "60",
    help: "If the game still runs this long after quitting, log where it is stuck and end it. 0 = wait forever.",
  }),
  ini({
    section: "Fixes", key: "DpiAware", tab: "fixes", label: "Ignore Windows display scaling", type: "bool",
    fallback: "1", help: "Full-screen and windowed modes use real pixels.",
  }),
  ini({
    section: "Fixes", key: "ExecutableSelfModifyingCode", tab: "fixes", label: "Survive DEP", type: "bool",
    fallback: "1", help: "Make the self-modifying rasteriser code executable, so Data Execution Prevention doesn't end the game.",
  }),
  ini({
    section: "Fixes", key: "MaxReportedPhysicalMemoryMB", tab: "fixes", label: "Reported memory", type: "int", min: 0,
    max: 4096, step: 64, unit: "MB", fallback: "512",
    help: "Largest physical memory reported to the game (it sizes the audio cache from it). 0 = the real value.",
  }),
  ini({
    section: "Fixes", key: "MaxReportedVideoMemoryMB", tab: "fixes", label: "Reported video memory", type: "int",
    min: 0, max: 4096, step: 64, unit: "MB", fallback: "1024",
    help: "Largest free video memory reported to the game. 0 = only prevent the 32-bit overflow.",
  }),

  // ---- Diagnostics ----
  ini({
    section: "Diagnostics", key: "ScreenshotKey", tab: "diagnostics", label: "Screenshot key", type: "vkey",
    fallback: "0", help: "Saves a PNG to Screenshots\\ in the game folder (Print Screen cannot capture the full-screen game).",
  }),
  ini({
    section: "Diagnostics", key: "TraceExit", tab: "diagnostics", label: "Log how the game ends", type: "bool",
    fallback: "1", help: "Log who ends the process or shows a message box.",
  }),
  ini({
    section: "Diagnostics", key: "TraceExceptions", tab: "diagnostics", label: "Log crashes", type: "bool",
    fallback: "1", help: "Log exceptions with registers and a scan of the stack.",
  }),
  ini({
    section: "Diagnostics", key: "ForwardDebugOutput", tab: "diagnostics", label: "Log the game's debug output",
    type: "bool", fallback: "0", help: "Copy the game's OutputDebugString text into BWDecomp.log.",
  }),
  ini({
    section: "Diagnostics", key: "TraceDirectX", tab: "diagnostics", label: "Trace DirectX calls", type: "bool",
    fallback: "0", help: "Count DirectDraw/Direct3D calls and write BWDecomp.directx.csv. Slows rendering slightly.",
  }),
  ini({
    section: "Log", key: "Debug", tab: "diagnostics", label: "Log every hook", type: "bool", fallback: "0",
    help: "List each installed hook in BWDecomp.log.",
  }),

  // ---- Advanced ----
  ini({
    section: "Hooks", key: "Disabled", tab: "advanced", label: "Disabled hooks", type: "text", fallback: "",
    help: "Original addresses (hex) whose decompiled replacement is switched off, separated by spaces, e.g. 0x0045c610 0x00749a70.",
  }),
];

export const fieldById = new Map(FIELDS.map((f) => [f.id, f]));

/** Values by field id, as strings, plus the resolution as game.ScreenW / game.ScreenH. */
export type Values = Record<string, string>;
