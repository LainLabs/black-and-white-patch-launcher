# Changelog

Semantic versioning. The version is in `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`
(`scripts/package.ps1` refuses if they differ) and tagged in git as `vX.Y.Z`. Each entry names the patch version it
bundles.

## Unreleased

- Development: the interface is built, tested, linted and type-checked with Vite+ 1.4 (`vp`), which replaces the
  separate Vite, Vitest and `tsc` steps. The built interface is unchanged. Building needs Node.js 22.18+, 24.11+ or 26+.
- The interface's code is formatted with Oxfmt, and `vp check` (part of every build) checks it.
- `npm run tauri dev` no longer watches `src-tauri\` for web changes, where Cargo writes its build output.

## 0.1.4

Bundles patch 0.11.1.

- Update repositories moved to `LainLabs/black-and-white-patch-launcher` and `LainLabs/black-and-white-patch-tool`.
  A saved setting that still names the old `konstantine-v` repositories follows the new ones.
- The default repositories are set in `updates.ini` instead of the code, so moving them needs no code change.

## 0.1.3

Bundles patch 0.11.0.

- Updates check two repositories: new launchers in `konstantine-v/black-and-white-patch-launcher` and new patches in
  `konstantine-v/black-and-white-patch-tool`, each with its own setting. A newer patch (`BWDecomp-<version>.zip`) is
  checked against its release's SHA-256 sums and installed into the game folder from the Updates page, without a new
  launcher. A patch newer than the bundled one is kept: setup and Play no longer offer to replace it with the older
  bundled one, and addons are checked against the installed patch.
- The single `updateRepo` setting of 0.1.x is replaced by `launcherRepo` and `patchRepo`.

## 0.1.2

Bundles patch 0.11.0.

- The launcher now says **patch** everywhere: the game is patched at run time, not decompiled. (The installation
  record `BWDecomp\Installed.txt` keeps its header, so the setup program still recognises launcher installations.)
- Setup saves the chosen game folder when it finishes. Choosing **Skip** on an installation that was already up to
  date left no folder saved, so the launcher showed setup again on every start.
- Updates come from the GitHub releases of `konstantine-v/black-and-white-patch-tool`: the newest release with a
  launcher setup program, its version read from the file name, so the patch's own releases there are ignored. A
  private or missing repository is reported as such.
- README rewritten, with the logo and screenshots.

## 0.1.0

First version. Bundles patch 0.11.0.

- Tauri 2 + React launcher carrying the patch's release files (embedded from `payload/` at build time).
- First-run wizard: finds the game (as the patch's setup program does), checks `runblack.exe` by SHA-256 with the same
  explanations, installs or updates the patch with the setup program's manifest (`BWDecomp\Installed.txt`), asks for
  administrator rights only when the folder needs them, sets `GameDir` when missing, then a quick setup.
- Takes over installations made by `BWDecompSetup.exe` (its uninstaller and Settings > Apps entry are removed).
- Play: starts `BWLaunch.exe`, follows the game until it ends, shows the end of `BWDecomp.log` when it fails.
  Optionally minimises or closes the launcher. `--play` starts the game without the window.
- Settings for all 27 `BWDecomp.ini` keys and the game's resolution, colour depth, detail and volumes, generated from
  one schema, with hints where settings interact. Edits keep the file's comments.
- Addons: texture packs (to `Textures\Replace\<id>\`) and settings presets (reverted when disabled), installed from a
  folder or zip, with patch version requirements. Unknown kinds are listed for later launchers.
- Updates from the GitHub releases of `konstantine-v/black-and-white-patch-tool` (the newest release with a launcher
  setup program; other releases there are ignored), verified against the release's SHA-256 sums before the new setup
  runs.
- Removing the patch from About (addons are disabled first; changed settings are kept).
