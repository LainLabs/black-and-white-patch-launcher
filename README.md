# Black & White Launcher

Installs the [Black & White decompilation](../BlackWhiteDecomp) into your copy of Lionhead's *Black & White* (2001),
lets you change its settings, and starts the game with its fixes and Direct3D 9 renderer.

This repository contains no game code or data. You need your own installed copy of the game (1.00, English, with the
unprotected `runblack.exe` from the disc's `crack` folder).

## For players

Run `BWLauncher-<version>-Setup.exe` (or the portable `BWLauncher-<version>-Portable.exe`). On first start the launcher:

1. finds the game folder (or lets you choose it) and checks `runblack.exe`, explaining what to do if it is the
   SafeDisc retail build or another version;
2. installs `BWLaunch.exe`, `BWDecomp.dll`, `BWDecomp.ini` and a `BWDecomp\` folder with the documentation, asking for
   administrator rights only if the folder needs them; none of the game's own files change;
3. offers the essentials: resolution, window mode, vertical sync, anti-aliasing, texture filtering.

After that it opens on **Play**. Also there:

- **Settings**: every option of `BWDecomp.ini` and the game's own resolution, detail and volume, with explanations.
  Comments in `BWDecomp.ini` are kept.
- **Addons**: texture packs and settings presets, enabled and disabled in one click.
- **Updates**: checks the GitHub releases of [black-and-white-patch-tool](https://github.com/konstantine-v/black-and-white-patch-tool); an update is checked against its SHA-256 sum before it runs.
- **About**: documentation, and removing the patch again (your changed settings, saves, screenshots and texture
  packs stay).

An installation made with the older `BWDecompSetup.exe` is picked up and updated in place.

`bw-launcher.exe --play` starts the game without opening the launcher.

## For developers

See [AGENTS.md](AGENTS.md): layout, how each part maps to the decomp, commands, rules and how to verify a change.

```powershell
. .\scripts\env.ps1
.\scripts\sync-payload.ps1      # the decomp's build output into payload\
npm install
npm run tauri dev
```

Black & White is a game by Lionhead Studios. This project is not affiliated with or endorsed by Lionhead Studios or
the game's publishers.
