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
  - Effect loops never sleep under 100 ms (10 Hz hard write cap; 15 Hz means key-off).
  - Brightness restored to max (2) on exit / SIGTERM / SIGINT / non-ctrl-c `drop`.
- **Effects** (`effects/`): Off, Static, Breathing (sine; speed changes cycle tempo),
  Strobing, Audio-reactive (cpal RMS, loopback with default-mic fallback), Battery
  (dims below threshold while discharging). Params are shared atomics — sliders take
  effect instantly without restarting loops.
- **Tauri plumbing**: ~15 IPC commands, system tray with Effects submenu, per-OS
  autostart (Linux `~/.config/autostart`, Windows winreg, macOS unsupported),
  Linux udev-rule installer (reloads udev + chmods live node).

### Frontend (`src/`)
- Dark-themed single-page UI: status header (power / mode / battery / hardware),
  effect cards with speed + sensitivity sliders, settings panel (install udev access,
  autostart toggle, close-to-tray). Typed IPC layer in `src/api.ts`.

## Verification
- `cargo check`: 0 errors / 0 warnings (all crates, Linux).
- `npm run build` (strict tsc + vite): passes.
- `npx tauri build --no-bundle`: full config + capabilities validated.
- Unit tests: 4/4 pass, including live write-quantization against real hardware.
- Live smoke tests on this machine: launched under Wayland and X11, tray initialized,
  driver detected, clean exit, and udev write access confirmed (node `-rw-rw-rw-`).
- Final release binary: 4.6 MB at `src-tauri/target/release/glowfx`.

## Known limitations
- `.deb` bundling impossible on this Arch host (no `dpkg`); AppImage needs the
  linuxdeploy download at build time and was not run.
- Windows driver is written but cannot be compile-verified on this Linux box.
- udev rule was installed manually via `sudo`; the app's in-app installer uses
  `pkexec` (needs a polkit agent / GUI prompt).

## Git history
- `484a79b` initial implementation
- `4ccf3f4` Wayland/WebKitGTK crash fix (`WEBKIT_DISABLE_COMPOSITING_MODE=1`)