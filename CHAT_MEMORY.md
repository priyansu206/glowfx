# GlowFX — Chat Memory

Context carried across sessions so work can resume without re-discovery.

## Project
- Root: `/home/priyansu/glowfx`. Frontend in `src/` (Vite root), backend in
  `src-tauri/`. Package manager: npm (no pnpm/cargo-tauri). Tauri CLI used via
  `npx @tauri-apps/cli`.
- Stack: Tauri v2, Rust 1.98, React 18 + TypeScript (strict), Vite 6.

## Target hardware (verified on this machine)
- Lenovo laptop, DMI module `VPC2004` (legacy Legion/LOQ backlight interface).
- Backlight: `/sys/class/leds/platform::kbd_backlight/brightness`, `max_brightness = 2`
  → only three discrete states: 0 (off), 1, 2. No arbitrary dimming.
- Originally root-only; user grants access once via a udev rule:
  `SUBSYSTEM=="leds", KERNEL=="*kbd_backlight", MODE="0666"` (file
  `/etc/udev/rules.d/99-lenovo-glowfx.rules`).

## Key decisions & gotchas
- **`pkexec` in this shell**: no polkit agent → times out. Elevation must happen in a
  real user session (app "Enable hardware access" button, or manual `sudo`).
- **Wayland crash**: WebKitGTK accelerated compositing ends in
  `Gdk-Message: Error 71 (Protocol error) dispatching to Wayland display.` Fix: set
  `WEBKIT_DISABLE_COMPOSITING_MODE=1` at startup when `WAYLAND_DISPLAY` is set
  (baked into `lib.rs::run()`, Linux-only). `npx tauri dev` was re-verified clean
  after the fix; stale scrollback can look like a regression.
- **cpal 0.15 / dasp types**: `cpal::Data::as_slice::<T>()` requires `T: SizedSample`;
  `SampleFormat` is `#[non_exhaustive]`. `Sample::to_float_sample()` returns
  `Self::Float` (f64 for f64/i64/u64), NOT f32 — audio.rs does explicit per-format
  numeric normalization instead.
- **std has no `AtomicF32`** → RMS level is stored as `AtomicU32` bit-pattern wrapper.
- **WMI (Windows driver)**: `WMIConnection` is `!Send` → per-thread `thread_local!`
  connections; namespace `ROOT\WMI`, class `Lenovo_SetBacklight`,
  `exec_instance_method::<Lenovo_SetBacklight, ()>`. Cannot be compiled on Linux
  (cfg-gated) — needs a Windows CI pass.
- **Breathing** uses fixed phase step (24°/step): the interval scales both smoothness
  and cycle tempo (slower speed = longer, coarser cycle), respecting the 100 ms floor.
- **Backlight restore** on exit targets brightness 2 (power state we booted with),
  regardless of in-app power toggle.
- Icicle generator: `/tmp/opencode/gen_icon.py` + `/tmp/opencode/glowfx-icon.png`;
  icons generated via `npx tauri icon`.
- `src-tauri/dist`-adjacent detail: `frontendDist: "../dist"` and `beforeBuildCommand:
  npm run build` — the built frontend in `/home/priyansu/glowfx/dist` is required at
  compile time (`generate_context!`); do not delete.

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

## Gotcha: build with `npx tauri build`, NOT plain `cargo build --release`
Plain `cargo build --release` does NOT enable Tauri's `custom-protocol` feature, so
`generate_context!` embeds nothing and the resulting binary loads `devUrl`
(`http://localhost:5173`) as if in dev mode → a window showing
**"Could not connect to localhost: Connection refused"** when Vite isn't running.
Always rebuild the runnable binary with `npx tauri build --no-bundle` (it enables
custom-protocol and embeds `dist/`). Verify with:
`strings src-tauri/target/release/glowfx | rg 'index-'` → should list the hashed
asset filenames, not just `localhost:5173`.
Also: never `pkill -f <pattern>` whose pattern appears in your own shell command
(it kills the shell and hangs the session); use `pkill -f 'patte[r]n'`.

## Open items
- Optional: `.deb` (needs Debian/Ubuntu host) and AppImage bundling test.
- Optional: Windows compile check (WMI + winreg paths).
- Optional: gentle-fade instead of hard {0,1,2} steps is impossible on this hardware.