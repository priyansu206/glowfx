# GlowFX — Chat Memory

## Project
- Root: `/home/priyansu/glowfx`. Frontend in `src/` (Vite root), backend in
  `src-tauri/`. Package manager: npm (no pnpm/cargo-tauri). Tauri CLI used via
  `npx @tauri-apps/cli`.
- Stack: Tauri v2, Rust 1.98, React 18 + TypeScript (strict), Vite 6.

## Target hardware (verified on this machine)
- Lenovo laptop, DMI module `VPC2004` (legacy Legion/LOQ backlight interface).
- Backlight: `/sys/class/leds/platform::kbd_backlight/brightness`, `max_brightness = 2`
  → only three discrete states: 0 (off), 1, 2. No arbitrary dimming.
- User grants access once via a udev rule:
  `SUBSYSTEM=="leds", KERNEL=="*kbd_backlight", MODE="0666"` (file
  `/etc/udev/rules.d/99-lenovo-glowfx.rules`).

## Effects
- Static, Breathing, Strobing, Audio (RMS loopback/mic + beat sync), Battery
  Guard (SOS blink at critical %, charge-complete pulse), Radar (rotating sweep with
  random target blips), Morse (message -> dots/dashes), Disco (random bursts),
  Schedule (time-of-day), Idle (DBus ScreenSaver auto-fade).
- Params: speed, sensitivity, static/min/max levels, waveform, audio_beat,
  auto_dim_minutes, critical_threshold, schedule hours, idle grace, battery
  threshold, tray_close. All live atomics — no effect restarts on change.
- Auto-dim override lives in `EffectShared::emit()`: force level 0 after N
  minutes of running a mode; `set_level()` is the un-dimmed raw write used for
  shutdown/restore. `run_effect` matches mode → effect module.

## Key decisions & gotchas
- **`pkexec` in this shell**: no polkit agent → times out. Elevation must happen
  in a real user session (app "Enable hardware access" button, or manual `sudo`).
- **Wayland crash**: WebKitGTK accelerated compositing ends in
  `Gdk-Message: Error 71 (Protocol error) dispatching to Wayland display.` Fixed by
  `WEBKIT_DISABLE_COMPOSITING_MODE=1` at startup when `WAYLAND_DISPLAY` is set
  (baked into `lib.rs::run()`, Linux-only).
- **Build must use `npx tauri build --no-bundle`, not plain `cargo build --release`.**
  Plain cargo build skips Tauri's `custom-protocol` feature, embeds no assets, and
  the binary opens `devUrl` (`http://localhost:5173`) → a window showing
  "Could not connect to localhost: Connection refused". Verify embed with
  `strings src-tauri/target/release/glowfx | rg 'index-'`.
- **cpal 0.15 / dasp types**: `cpal::Data::as_slice::<T>()` requires `T: SizedSample`;
  `SampleFormat` is `#[non_exhaustive]`. `Sample::to_float_sample()` returns
  `Self::Float` (f64 for f64/i64/u64), NOT f32 — audio.rs does explicit per-format
  numeric normalization instead.
- **std has no `AtomicF32`** → RMS level stored as `AtomicU32` bit-pattern wrapper.
- **WMI (Windows driver)**: `WMIConnection` is `!Send` → per-thread `thread_local!`
  connections; namespace `ROOT\WMI`, class `Lenovo_SetBacklight`,
  `exec_instance_method::<Lenovo_SetBacklight, ()>`. Cannot be compiled on Linux
  (cfg-gated) — needs a Windows CI pass.
- **Breathing** uses fixed phase step (24°/step): the interval scales both smoothness
  and cycle tempo, respecting the 100 ms floor. Strobe/blink "decay" waveform is a
  triangle ramp (max→min→max).
- **Backlight restore** on exit targets brightness 2, regardless of in-app power toggle.
- `frontendDist: "../dist"` with `beforeBuildCommand: npm run build` — the built
  frontend in `/home/priyansu/glowfx/dist` is required at compile time
  (`generate_context!`).
- Icon source: `/tmp/opencode/gen_icon.py` → `/tmp/opencode/glowfx-icon.png`,
  converted with `npx tauri icon`.

## Environment
- Arch Linux, Wayland (`XDG_SESSION_TYPE=wayland`, `WAYLAND_DISPLAY=wayland-1`,
  `DISPLAY=:0` available for X11 backend).
- No `dpkg`/`debuild` (no `.deb`); AppImage bundling downloads tools at build time.
- `npm run build`, `npm run tauri build` from `/home/priyansu/glowfx`; cargo work in
  `src-tauri/`. `WINIT_UNIX_BACKEND=x11 GDK_BACKEND=x11` forces X11 fallback if ever
  needed for debugging.

## Run commands
```bash
/home/priyansu/glowfx/src-tauri/target/release/glowfx   # run (no elevation needed)
cd /home/priyansu/glowfx && npx tauri dev               # dev with HMR
cd /home/priyansu/glowfx/src-tauri && cargo test --release
```

## Open items
- `.deb` needs a Debian/Ubuntu host; AppImage bundling test may be run later.
- Windows compile check for the WMI + winreg paths.
- Gentle-fade instead of hard {0,1,2} steps is impossible on this hardware.