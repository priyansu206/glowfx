import { useCallback, useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import StatusHeader from "./components/StatusHeader";
import EffectCard from "./components/EffectCard";
import SpeedSlider from "./components/SpeedSlider";
import AudioSens from "./components/AudioSens";
import SettingsPanel from "./components/SettingsPanel";
import {
  emptyBattery,
  getStatus,
  quitApp,
  setMode,
  setPower,
  setSensitivity,
  setSpeed,
  setStaticLevel,
  setAutostart,
  setTrayClose,
  type Mode,
  type StatusInfo,
} from "./api";

interface ModeMeta {
  title: string;
  description: string;
  icon: JSX.Element;
}

const MODE_META: Record<Mode, ModeMeta> = {
  static: {
    title: "Static",
    description: "Fixed backlight level",
    icon: (
      <svg viewBox="0 0 24 24" width="26" height="26">
        <rect x="4" y="8" width="16" height="8" rx="2" fill="currentColor" />
      </svg>
    ),
  },
  breathing: {
    title: "Breathing",
    description: "Slow quantized fade loop",
    icon: (
      <svg viewBox="0 0 24 24" width="26" height="26">
        <path
          d="M2 12h4l2.5-6 3 12 3-9 1.5 3H22"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.8"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      </svg>
    ),
  },
  strobing: {
    title: "Strobing",
    description: "High-frequency flash toggle",
    icon: (
      <svg viewBox="0 0 24 24" width="26" height="26">
        <path d="M13 2 4 14h6l-1 8 9-12h-6l1-8z" fill="currentColor" />
      </svg>
    ),
  },
  audio: {
    title: "Audio Visualizer",
    description: "React to system/mic sound",
    icon: (
      <svg viewBox="0 0 24 24" width="26" height="26">
        <rect x="3" y="10" width="4" height="7" rx="1" fill="currentColor" />
        <rect x="10" y="5" width="4" height="14" rx="1" fill="currentColor" />
        <rect x="17" y="8" width="4" height="9" rx="1" fill="currentColor" />
      </svg>
    ),
  },
  battery: {
    title: "Battery Guard",
    description: "Low-battery flash alert",
    icon: (
      <svg viewBox="0 0 24 24" width="26" height="26">
        <rect x="2" y="7" width="17" height="10" rx="2" fill="none" stroke="currentColor" strokeWidth="1.8" />
        <rect x="4" y="9" width="8" height="6" rx="1" fill="currentColor" />
        <path d="M21 10v4" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
      </svg>
    ),
  },
};

const MODE_ORDER: Mode[] = ["static", "breathing", "strobing", "audio", "battery"];

const EMPTY_STATUS: StatusInfo = {
  power: false,
  mode: "off",
  static_level: 2,
  interval_ms: 150,
  speed: 4,
  sensitivity: 60,
  driver_supported: false,
  driver_path: "",
  driver_error: null,
  audio_error: null,
  battery: null,
  autostart: false,
  tray_close: false,
  os: "unknown",
};

export default function App() {
  const [status, setStatus] = useState<StatusInfo>(EMPTY_STATUS);
  const [busy, setBusy] = useState<Set<string>>(new Set());
  const trayCloseRef = useRef(status.tray_close);
  trayCloseRef.current = status.tray_close;

  const refresh = useCallback(async () => {
    try {
      setStatus(await getStatus());
    } catch {
      /* frontend just mounted before backend is ready */
    }
  }, []);

  useEffect(() => {
    refresh();
    const t = window.setInterval(refresh, 2000);
    return () => window.clearInterval(t);
  }, [refresh]);

  useEffect(() => {
    const unlisten = getCurrentWindow().onCloseRequested(async (event) => {
      if (trayCloseRef.current) {
        event.preventDefault();
        await getCurrentWindow().hide();
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const run = useCallback(
    async (key: string, fn: () => Promise<unknown>) => {
      setBusy((s) => new Set(s).add(key));
      try {
        await fn();
        await refresh();
      } catch (e) {
        console.error(key, e);
      } finally {
        setBusy((s) => {
          const next = new Set(s);
          next.delete(key);
          return next;
        });
      }
    },
    [refresh]
  );

  const power = status.power;
  const currentMode = (MODE_ORDER.includes(status.mode as Mode) ? status.mode : "off") as Mode | "off";

  const handlePower = () => run("power", () => setPower(!power));
  const handleMode = (m: Mode) => run(`mode:${m}`, () => setMode(m));
  const handleSpeed = (v: number) => run("speed", () => setSpeed(v));
  const handleSens = (v: number) => run("sens", () => setSensitivity(v));
  const handleStatic = (v: number) => run("static", () => setStaticLevel(v));
  const handleAutostart = (v: boolean) => run("auto", () => setAutostart(v));
  const handleTray = (v: boolean) => run("tray", () => setTrayClose(v));
  const handleQuit = () => quitApp();

  const battery = status.battery ?? emptyBattery();

  return (
    <div className="app">
      <StatusHeader
        power={power}
        driverSupported={status.driver_supported}
        driverPath={status.driver_path}
        battery={status.battery ?? null}
        mode={status.mode}
        onPower={handlePower}
      />

      <main className="content">
        <section className="section">
          <div className="section-title">Effects</div>
          <div className="effect-grid">
            {MODE_ORDER.map((m) => (
              <EffectCard
                key={m}
                mode={m}
                title={MODE_META[m].title}
                description={MODE_META[m].description}
                icon={MODE_META[m].icon}
                selected={currentMode === m}
                disabled={!status.driver_supported || busy.has("power")}
                onClick={() => handleMode(m)}
              />
            ))}
          </div>
          <div className="controls-grid">
            <SpeedSlider value={status.speed} disabled={!power || !status.driver_supported} onChange={handleSpeed} />
            <AudioSens
              value={status.sensitivity}
              mode={status.mode}
              disabled={!power || !status.driver_supported}
              onChange={handleSens}
            />
            <div className={`control-row${currentMode !== "static" ? " disabled" : ""}`}>
              <span className="control-label">Static level</span>
              <div className="level-picker">
                {[0, 1, 2].map((l) => (
                  <button
                    key={l}
                    type="button"
                    className={`level-btn${status.static_level === l ? " active" : ""}`}
                    disabled={currentMode !== "static" || !power || !status.driver_supported}
                    onClick={() => handleStatic(l)}
                  >
                    {["Off", "Low", "High"][l]}
                  </button>
                ))}
              </div>
            </div>
          </div>
          {status.audio_error && status.mode === "audio" && (
            <div className="install-result err">Audio: {status.audio_error}</div>
          )}
        </section>

        <SettingsPanel
          autostart={status.autostart}
          trayClose={status.tray_close}
          disabled={busy.has("auto") || busy.has("tray")}
          onAutostart={handleAutostart}
          onTrayClose={handleTray}
          onQuit={handleQuit}
        />
      </main>

      <footer className="footer">
        GlowFX v0.1 · {status.os} · writes rate-limited to ≤10 Hz · backlight state {battery.percent}%
      </footer>
    </div>
  );
}