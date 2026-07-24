<div align="center">

# DaVinci Resolve Uninstaller

**Completely remove every trace of DaVinci Resolve from Windows — install files, config, caches, shortcuts and registry keys — in one click.**

[![Release](https://img.shields.io/github/v/release/ExoticGamerrrYT/davinci-uninstaller?display_name=tag)](https://github.com/ExoticGamerrrYT/davinci-uninstaller/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/ExoticGamerrrYT/davinci-uninstaller/total)](https://github.com/ExoticGamerrrYT/davinci-uninstaller/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
![Platform](https://img.shields.io/badge/platform-Windows%2010%20%2F%2011-0078D6?logo=windows)
![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-24C8DB?logo=tauri)

</div>

---

Blackmagic's own uninstaller often leaves gigabytes of caches, config folders,
Start Menu entries and registry keys behind — enough to block a clean reinstall.
This tool finds and removes all of it. Your **projects and databases are kept by
default**; deleting them is a separate, clearly-warned opt-in.

## Features

- **Read-only scan first** — see exactly what will be removed before anything is touched.
- **Complete cleanup** — install folders, `ProgramData`/`AppData` config & cache, Documents, Start Menu folders, desktop shortcuts, and all Resolve registry keys.
- **Projects are safe by default** — an explicit toggle (off by default) is required to delete projects, the Resolve Disk Database and PostgreSQL.
- **Runs the official uninstaller too** — if Resolve is still registered, Blackmagic's uninstaller is invoked first, then the leftovers are swept.
- **Live progress log** — every step is reported in real time; all helper processes run hidden (no flashing console windows).
- **Portable** — a single `.exe`, no installation. The only dependency (WebView2) ships with Windows 10/11.

## Download & use

1. Grab the latest **`DaVinci-Resolve-Uninstaller-*.zip`** from the [Releases page](https://github.com/ExoticGamerrrYT/davinci-uninstaller/releases/latest) and extract it.
2. Right-click **`DaVinci Resolve Uninstaller.exe`** → **Run as administrator** (required to delete system files and registry keys).
3. Review the scan, tick *"Also delete projects"* only if you want your projects gone, then click **Uninstall DaVinci Resolve**.
4. A Windows restart is recommended afterwards to release any files that were locked.

## What gets removed

| Category | Locations |
|---|---|
| Install | `C:\Program Files\Blackmagic Design\DaVinci Resolve` |
| Config & cache | `%PROGRAMDATA%`, `%APPDATA%`, `%LOCALAPPDATA%`, Documents, Public Documents (`Blackmagic Design\DaVinci Resolve`) |
| Shortcuts | Start Menu `Blackmagic Design` folders (all users + current user), desktop `.lnk` shortcuts |
| Registry | `HKLM` & `HKCU` `SOFTWARE\Blackmagic Design\DaVinci Resolve` (incl. `WOW6432Node`) |
| Projects *(opt-in)* | `Resolve Disk Database`, PostgreSQL service + data |

## Safety

- The **scan never deletes** — removal happens only after you confirm.
- Every deletion passes an allowlist: only paths under Blackmagic / PostgreSQL, or Resolve `.lnk` shortcuts, and never a shallow system path.
- With *"Also delete projects"* **off**, the `Resolve Disk Database` subtrees are preserved even though they live inside the app folders.
- Only the Windows system drive is touched — config, registry and databases always live there.

> [!WARNING]
> Deleting projects is **irreversible**. Back up anything you care about first.

## Build from source

Requires [Node.js](https://nodejs.org) + [pnpm](https://pnpm.io), the [Rust toolchain](https://rustup.rs), and the [Tauri prerequisites](https://tauri.app/start/prerequisites/).

```sh
pnpm install
pnpm tauri dev                                     # run in development

cargo test --manifest-path src-tauri/Cargo.toml    # backend logic tests

pnpm tauri build --no-bundle                        # build portable release exe
pwsh ./package.ps1                                  # zip it into deploy/
```

## Tech stack

Tauri 2 · SvelteKit (Svelte 5, runes) · Tailwind CSS 4 · Rust (`winreg`).

## License

[MIT](LICENSE) © ExoticGamerrrYT
