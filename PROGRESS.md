# GlowFX — Progress Log

> Lenovo LOQ/Legion single-zone keyboard-backlight controller app.
> Rust + Tauri v2 backend, React + TypeScript frontend.

## Status: COMPLETE (working on real hardware)

## What was delivered

### Backend (`src-tauri/`)
- **Driver HAL** (`driver/`): `BacklightDriver` trait with three impls —
  `LinuxDriver` (sysfs `/sys/class/leds/platform::kbd_backlight/brightness`),
  `WindowsDriver` (WMI `Lenovo_SetBacklight`/`SetBacklight`, cfg-gated), and a
  `NullDriver` fallback. All writes serialized through a single shared `Arc` mutex.
- **Safety constraints**:
  - Brightness hard-quantized to the hardware states `{0, 1, 2}`.
  - Effect loops never sleep under 100 ms (10 Hz hard write cap).
  - Brightness restored to max (2) on exit / SIGTERM / SIGINT / non-ctrl-c `drop`.
- **Effects** (`effects/`): Off, Static, Breathing (sine, min/max ranged), Strobing
  (square/decay), Audio (RMS loopback + beat-sync mode), Battery Guard (SOS at
  critical level, charge-complete pulse), Candle (ember flicker), Heartbeat
  (lub-dub), Blink, Disco, Schedule (time-of-day), Idle (DBus ScreenSaver auto-fade).
  Params are shared atomics — sliders take effect instantly without restarting loops.
- **Tuning**: per-mode min/max levels, square/decay waveforms, audio beat sync,
  central auto-dim timeout, critical battery %, schedule hours, idle grace.
- **Tauri plumbing**: ~24 IPC commands, system tray with Effects submenu, per-OS
  autostart (Linux `~/.config/autostart`, Windows winreg, macOS unsupported),
  Linux udev-rule installer (reloads udev + chmods live node).
- Dependencies in use: `cpal`, `ctrlc`, `chrono` (schedule), `zbus` (idle detection).

### Frontend (`src/`)
- Dark-themed single-page UI: status header (power / mode / battery / hardware),
  effect cards for all 11 modes, speed + sensitivity sliders, per-mode options
  (level range, waveform, reactivity), settings panel (udev access, autostart,
  tray-close, auto-dim, critical %, schedule hours, idle grace). Typed IPC layer in
  `src/api.ts`.

## Verification
- `cargo check`: 0 errors / 0 warnings (all crates, Linux).
- `npm run build` (strict tsc + vite): passes.
- `npx tauri build --no-bundle`: full config + capabilities validated, UI embedded.
- Unit tests: 6/6 pass, including live write-quantization and live runs of the new
  effect loops (Candle/Blink/Heartbeat/Disco) against the real backlight.
- Live smoke tests on this machine: launched under Wayland and X11, tray initialized,
  driver detected, clean exit, udev write access confirmed (node `-rw-rw-rw-`).
- Final release binary: 4.8 MB at `src-tauri/target/release/glowfx`.

## Known limitations
- `.deb` bundling impossible on this Arch host (no `dpkg`); AppImage needs the
  linuxdeploy download at build time and was not run.
- Windows driver is written but cannot be compile-verified on this Linux box.
- udev rule was installed manually via `sudo`; the app's in-app installer uses
  `pkexec` (needs a polkit agent / GUI prompt).
- Idle effect requires a session DBus for real idle time; safely no-ops otherwise.

## Git history
- `484a79b` initial implementation
- `4ccf3f4` Wayland/WebKitGTK crash fix (`WEBKIT_DISABLE_COMPOSITING_MODE=1`)
- `8c59b0b` progress + chat-memory docs; build gotcha recorded
- `daab449` six new effects + tuning enhancements