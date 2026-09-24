# GlowFX

A Lenovo LOQ/Legion keyboard-backlight control suite. Reactive single-zone
keyboard backlight effects with a dark-themed desktop UI.

**Stack:** Tauri v2 (Rust) backend · React 18 + TypeScript (strict) · Vite 6 frontend.

## Features

- **12 effects**: Off, Static, Breathing, Strobing, Audio (mic/loopback + beat sync),
  Battery Guard (SOS at critical %, charge pulse), Storm (random lightning bursts),
  Ripple (sonar ping), Morse (blinks a typed message), Schedule (time-of-day),
  Idle (D-Bus screen-saver auto-fade), and Disco.
- **Live tuning**: speed, sensitivity, level range, waveform — applied instantly via
  shared atomics, no effect restarts.
- **Safety**: brightness quantized to hardware states `{0, 1, 2}`; writes capped at
  10 Hz; brightness restored on exit.
- **System tray** with effects submenu, per-OS autostart, Linux udev-rule installer.

## Requirements

### All platforms

- [Node.js](https://nodejs.org) 18+ (npm)
- [Rust](https://rustup.rs) (stable, 1.70+)
- [Tauri v2 prerequisites](https://tauri.app/start/prerequisites/) for your OS

Build-time prerequisite for Linux: `webkit2gtk`, `libappindicator`, and friends
(see the Tauri docs). On Windows 10/11, WebView2 is preinstalled. macOS needs Xcode
command line tools.

### Supported hardware / OS

| OS      | Support                                        |
| ------- | ---------------------------------------------- |
| Linux   | Full (sysfs `platform::kbd_backlight`)         |
| Windows | Full (WMI `Lenovo_SetBacklight`)               |
| macOS   | Compiles, UI runs, backlight **not supported** |

## Getting started

```bash
git clone https://github.com/priyansu206/glowfx.git
cd glowfx
npm install
npm run tauri dev      # development with hot reload
```

### Building a release

```bash
npm run tauri build
```

Installers are emitted to `src-tauri/target/release/bundle/`:

- **Linux**: AppImage, deb, rpm
- **Windows**: MSI, NSIS (`GlowFX_0.1.0_x64-setup.exe`)
- **macOS**: DMG

> Use `npm run tauri build -- --no-bundle` to skip bundling when a system package
> manager (e.g. `dpkg`) is missing — a plain `cargo build` will not embed the UI.

## Linux: hardware access (one time)

The backlight node is root-owned, so add a udev rule (the app has an in-app
installer that uses `pkexec`):

```
SUBSYSTEM=="leds", KERNEL=="*kbd_backlight", MODE="0666"
```

Save it as `/etc/udev/rules.d/99-lenovo-glowfx.rules` and reload:

```bash
sudo udevadm control --reload-rules && sudo udevadm trigger
```

## Project layout

```
src/            React + TS frontend (Vite root)
src-tauri/
  src/driver/   Backlight HAL: linux.rs, windows.rs, macos.rs
  src/effects/  Per-mode effect loops
  src/tray.rs   System tray
  Cargo.toml    Backend manifest (wmi/winreg cfg-gated for Windows)
```

## Known limitations

- Hardware only supports discrete brightness 0/1/2 — no arbitrary dimming.
- `.deb` bundling requires a Debian/Ubuntu host.
- Windows driver is written but compile-verified only on CI; macOS has no backlight
  interface and degrades gracefully.

## Development notes

- `npm run tauri dev` (or `npx @tauri-apps/cli dev`) from the repo root.
- Backend unit tests: `cd src-tauri && cargo test --release`.
- Details and past gotchas live in `CHAT_MEMORY.md` / `PROGRESS.md`.