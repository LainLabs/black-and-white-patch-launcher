# Changelog

Semantic versioning. The version is in `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`
(`scripts/package.ps1` refuses if they differ) and tagged in git as `vX.Y.Z`. Each entry names the decompilation it
bundles.

## 0.1.0

First version. Bundles decompilation 0.11.0.

- Tauri 2 + React launcher carrying the decompilation's release files (embedded from `payload/` at build time).
- First-run wizard: finds the game (as the decomp's setup does), checks `runblack.exe` by SHA-256 with the same
  explanations, installs or updates the patch with the setup program's manifest (`BWDecomp\Installed.txt`), asks for
  administrator rights only when the folder needs them, sets `GameDir` when missing, then a quick setup.
- Takes over installations made by `BWDecompSetup.exe` (its uninstaller and Settings > Apps entry are removed).
- Play: starts `BWLaunch.exe`, follows the game until it ends, shows the end of `BWDecomp.log` when it fails.
  Optionally minimises or closes the launcher. `--play` starts the game without the window.
- Settings for all 27 `BWDecomp.ini` keys and the game's resolution, colour depth, detail and volumes, generated from
  one schema, with hints where settings interact. Edits keep the file's comments.
- Addons: texture packs (to `Textures\Replace\<id>\`) and settings presets (reverted when disabled), installed from a
  folder or zip, with decomp version requirements. Unknown kinds are listed for later launchers.
- Updates from GitHub releases, verified against the release's SHA-256 sums before the new setup runs.
- Removing the patch from About (addons are disabled first; changed settings are kept).
