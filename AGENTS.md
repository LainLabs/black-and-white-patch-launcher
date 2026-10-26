# AGENTS.md — Black & White Launcher

Guide for agents (and people) working on this repository. Read it before changing anything.

## What this is

A desktop launcher for the **Black & White decompilation** (sibling repo `..\BlackWhiteDecomp`). One executable:

1. carries the decompilation's release files (`BWLaunch.exe`, `BWDecomp.dll`, `BWDecomp.ini`, player docs), embedded at
   build time;
2. on first run, walks the player through finding the game, checking `runblack.exe` and installing those files;
3. afterwards is where the player presses **Play**, edits settings, manages addons and gets updates.

It replaces the decomp's `BWDecompSetup.exe` as the player-facing download. Setup still exists in the decomp repo
for `/quiet` installs, and both write the **same manifest**, so either can update or remove what the other installed.

Stack: **Tauri 2** (Rust backend, `src-tauri/`) + **React 19 / TypeScript / Vite / Tailwind 4** (`src/`), TanStack
Query for backend reads, Zustand for UI state. Tauri 3 is still alpha: stay on Tauri 2 until it is stable.

## The decomp is the source of truth

This repo **consumes** the decomp; it never changes it. When the decomp changes, the launcher follows. Mapping:

| Launcher | Mirrors (in `BlackWhiteDecomp`) |
|---|---|
| `scripts/sync-payload.ps1` file list, Keep/Replace | `src/setup/Payload.cpp`, `package.ps1` |
| `src-tauri/src/install.rs` (manifest `BWDecomp\Installed.txt`, write-then-move, Keep files, removal) | `src/setup/Install.cpp` |
| `src-tauri/src/game.rs` (folder search order, SHA-256 check, messages, running check) | `src/setup/System.cpp` (`FindGameFolder`, `IsGameRunning`), `src/core/GameExecutable.cpp` |
| Supported / SafeDisc hashes | `src/core/Image.h`, copied into `payload/payload.json` by sync-payload (never hard-code new ones) |
| `src-tauri/src/elevate.rs` (rights only when needed, file work only) | `src/setup/Setup.cpp` `RunElevated`, `System.cpp` `NeedsElevation` |
| `src-tauri/src/registry.rs` (`GameDir`, `BWSetup` values, legacy uninstall key) | `src/setup/System.cpp`, `docs/FIXES.md` |
| `src/settings/schema.ts` (every INI key, default, help text) | `BWDecomp.ini`, `src/core/Config.h`, `src/core/Config.cpp` |
| `src/settings/logic.ts` `hints()` (how settings interact) | `src/core/Config.cpp`, `docs/RENDERER.md`, `docs/TEXTURES.md` |
| Starting the game | `src/launcher/Launcher.cpp`: `BWLaunch.exe [--game <exe>] [-- <args>]` injects the DLL, then exits. The launcher runs it and does **not** reimplement injection. |

## Layout

```
AGENTS.md README.md CHANGELOG.md
scripts/env.ps1            puts cargo + MSVC x64 tools on PATH (dot-source it)
scripts/sync-payload.ps1   decomp build\Release + docs -> payload\ (+ payload.json)
scripts/package.ps1        sync -> tests -> tauri build -> dist\ (Setup, Portable, SHA256SUMS)
payload/                   gitignored; embedded by src-tauri/build.rs
examples/addons/           sample addons (a preset and a texture-pack layout)
src/                       React UI
  main.tsx                 entry; installs lib/devMock.ts in a plain browser during development
  app/App.tsx              wizard vs. shell, side navigation, game events, startup update check
  pages/                   Wizard, Home (Play), Settings, Addons, Updates, About
  components/              ui.tsx (kit), SettingField.tsx (schema-driven rows), Backdrop.tsx (original art)
  settings/schema.ts       THE settings table; schema.test.ts checks it against the shipped BWDecomp.ini
  settings/logic.ts        snapshot <-> form values, change sets, interaction hints, virtual-key codes
  lib/api.ts               typed wrappers for every Rust command (keep in step with commands.rs)
  lib/queries.ts           TanStack Query hooks and cache keys
  lib/store.ts             current page, run state, wizard flag
  lib/devMock.ts           fake backend for browser previews (dev only)
src-tauri/
  build.rs                 payload/payload.json -> OUT_DIR/payload.rs (include_bytes! of each file)
  tauri.conf.json          window, CSP, NSIS bundle (per-user, WebView2 bootstrapper)
  capabilities/default.json  plugin permissions for the window
  src/main.rs, lib.rs      CLI modes (--elevated, --play) and the Tauri builder
  src/commands.rs          every command the UI calls
  src/{game,install,elevate,fsops,ini,registry,settings,display,launch,addons,updates,state,paths,payload,error}.rs
```

## Commands

```powershell
. .\scripts\env.ps1                 # once per shell: cargo + MSVC on PATH
.\scripts\sync-payload.ps1          # needs ..\BlackWhiteDecomp\build\Release (add -Build to build it)
npm install
npm run tauri dev                   # the app with hot reload (real backend)
npm run dev                         # UI only in a browser at http://localhost:1420 with the fake backend
                                    #   ?mock=fresh shows the first-run wizard
npx tsc --noEmit; npx vitest run    # UI checks
cd src-tauri; cargo test            # backend tests (temp folders only; never touch a real game folder)
.\scripts\package.ps1               # release files in dist\
```

Toolchain: Rust stable MSVC (`rustup`), Visual Studio 2022 C++ build tools (x64 for the launcher; the decomp needs
x86/Win32), Node 20+. Without `payload\` the launcher still builds, for UI work, and says it has nothing to install.

## Rules

- **Never ship or commit anything from the game** (code, data, art, icons). The art in `Backdrop.tsx` and the icon in
  `src-tauri/icon.svg` are original. `payload/` holds only the decomp's own files and is gitignored.
- **Keep the manifest compatible** with `src/setup/Install.cpp`: header `# Black & White decompilation <version>`,
  lines `<sha256 lower-case> R|K <path with \>`, CRLF. Paths read from a manifest are checked with
  `paths::is_safe_relative_path` before anything is deleted.
- **File safety:** every write into the game folder goes through `install::write_replacing` (write next to the target,
  flush, move over it) or `fsops` (relative, checked targets). Never delete outside the game folder; never delete a
  Keep file the player changed (hash differs).
- **Administrator rights only when needed**, only for file work: `elevate::execute_with_rights` /
  `fsops::apply_with_rights` re-run the launcher with `--elevated <request.json> <result.json>`. Registry values and
  launcher state stay in the player's own (non-elevated) process.
- **Refuse while the game runs** from the folder (`game::is_game_running`) before installing, removing or changing
  addons.
- **Settings:** `BWDecomp.ini` is edited line by line (`ini::set`), keeping every comment, because the shipped file
  documents itself. Registry values in `BWSetup` are DWORDs; an existing REG_SZ keeps its type.
- **Slow work off the UI thread:** commands that hash, copy or use the network are `async` and use `blocking(...)`.
- Match the surrounding style: short doc comments that say *why*, error messages a player can act on (with the path),
  British spelling in UI text as in the decomp's docs.

## Common changes

**The decomp added or changed an INI key.** Add or adjust one entry in `src/settings/schema.ts` (section, key, tab,
label, help, type, range, `fallback` = the shipped default). `npx vitest run` fails until every key in the shipped
`BWDecomp.ini` is in the schema with the same default, and nothing in the schema is missing from the file. Add an
interaction to `hints()` in `settings/logic.ts` if the key depends on another.

**A new decomp release.** In the decomp: build. Here: `.\scripts\sync-payload.ps1`, run the tests, bump the launcher
version (`package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`: package.ps1 refuses if they differ), add
a CHANGELOG entry naming the decomp version, `.\scripts\package.ps1`, tag `vX.Y.Z`, publish `dist\BWLauncher-*` on the
GitHub release (the update check needs `-Setup.exe` and `-SHA256SUMS.txt`).

**A new Rust command.** Write it in `commands.rs` (return `error::Result<T>`; `T: Serialize` with
`#[serde(rename_all = "camelCase")]`), register it in `lib.rs` `generate_handler!`, add the typed wrapper in
`src/lib/api.ts`, and a fake answer in `src/lib/devMock.ts`.

**A new addon kind.** Addons live in `%APPDATA%\BWLauncher\addons\<id>\addon.json` (`id`, `name`, `version`, `kind`,
optional `author`, `description`, `homepage`, `requires.decomp` semver range, and kind-specific fields).
1. Add the kind to `KINDS` in `src-tauri/src/addons.rs`, its content count in `list()`, its deploy step in `enable()`
   and its exact undo in `disable()`. Record everything it writes in the `Deployment` (files and/or setting
   changes) so disabling removes exactly that.
2. Add it to `ADDON_KINDS` in `src/pages/Addons.tsx` (label, icon, unit).
3. Add an example under `examples/addons/` and tests in `addons.rs`.
Unknown kinds are listed with "needs a newer launcher" instead of failing, so addons can ship ahead of support.
Kinds that would load code into the game (DLL plugins, scripts) need a plugin interface in the decomp first: design it
there, then add the kind here.

**UI.** Pages are plain components switched by `useStore().page`. Use the kit in `components/ui.tsx` and the palette in
`styles.css` (`ink`, `ivory`, `gold`, `good`, `evil`). Check the page in a browser with `npm run dev` (fake backend)
before trying it in the app.

## Verifying a change

1. `npx tsc --noEmit`, `npx vitest run`, `cargo test` (in `src-tauri`).
2. `npm run dev` and look at the affected pages (both `/` and `/?mock=fresh` for wizard changes).
3. For backend changes, `npm run tauri dev` against a **scratch copy** of a game folder: install, change settings
   (check the `BWDecomp.ini` diff and `HKCU\Software\Lionhead Studios Ltd\Black & White\BWSetup`), enable and disable
   an example addon, remove the patch. Play and check `BWDecomp.log` says the DLL attached.
4. Release changes: `.\scripts\package.ps1`, install `dist\BWLauncher-<v>-Setup.exe`, repeat the Play check.

Launcher state for testing lives in `%APPDATA%\BWLauncher` (`launcher.json`, `deployed.json`, `addons\`); delete it
to see the first-run wizard again.
