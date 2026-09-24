import { useCallback, useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import StatusHeader from "./components/StatusHeader";
import EffectCard from "./components/EffectCard";
import SpeedSlider from "./components/SpeedSlider";
import AudioSens from "./components/AudioSens";
import ModeOptions from "./components/ModeOptions";
import SettingsPanel from "./components/SettingsPanel";
import {
  emptyBattery,
  getStatus,
  quitApp,
  setAudioBeat,
  setAutostart,
  setAutoDim,
  setCriticalThreshold,
  setMaxLevel,
  setMinLevel,
  setMode,
  setPower,
  setScheduleHours,
  setSensitivity,
  setSpeed,
  setStaticLevel,
  setTrayClose,
  setWaveform,
  setIdleGrace,
  setMorseText,
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
    description: "Low-battery SOS + charge pulses",
    icon: (
      <svg viewBox="0 0 24 24" width="26" height="26">
        <rect x="2" y="7" width="17" height="10" rx="2" fill="none" stroke="currentColor" strokeWidth="1.8" />
        <rect x="4" y="9" width="8" height="6" rx="1" fill="currentColor" />
        <path d="M21 10v4" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
      </svg>
    ),
  },
  storm: {
    title: "Storm",
    description: "Random lightning bursts",
    icon: (
      <svg viewBox="0 0 24 24" width="26" height="26">
        <path d="M13 2 4 14h6l-1 8 9-12h-6l1-8z" fill="currentColor" />
        <path d="M19 5v6M22 8h-6" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" />
      </svg>
    ),
  },
  ripple: {
    title: "Ripple",
    description: "Sonar ping with decay dwell",
    icon: (
      <svg viewBox="0 0 24 24" width="26" height="26">
        <circle cx="12" cy="12" r="9" fill="none" stroke="currentColor" strokeWidth="1.6" />
        <circle cx="12" cy="12" r="5" fill="none" stroke="currentColor" strokeWidth="1.6" opacity="0.7" />
        <circle cx="12" cy="12" r="1.5" fill="currentColor" />
      </svg>
    ),
  },
  radar: {
    title: "Radar",
    description: "Rotating sweep with target blips",
    icon: (
      <svg viewBox="0 0 24 24" width="26" height="26">
        <circle cx="12" cy="12" r="9" fill="none" stroke="currentColor" strokeWidth="1.6" />
        <circle cx="12" cy="12" r="5" fill="none" stroke="currentColor" strokeWidth="1.6" opacity="0.5" />
        <path d="M12 12 17 7" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
        <circle cx="15.5" cy="9.5" r="2" fill="currentColor" />
      </svg>
    ),
  },
  morse: {
    title: "Morse Code",
    description: "Blink a typed message",
    icon: (
      <svg viewBox="0 0 24 24" width="26" height="26">
        <rect x="3" y="8" width="8" height="3" rx="1.5" fill="currentColor" />
        <rect x="3" y="14" width="18" height="3" rx="1.5" fill="currentColor" />
      </svg>
    ),
  },
  disco: {
    title: "Disco",
    description: "Random burst party",
    icon: (
      <svg viewBox="0 0 24 24" width="26" height="26">
        <circle cx="7" cy="12" r="5" fill="currentColor" opacity="0.6" />
        <circle cx="17" cy="12" r="3.5" fill="currentColor" />
      </svg>
    ),
  },
  schedule: {
    title: "Schedule",
    description: "Time-of-day brightness",
    icon: (
      <svg viewBox="0 0 24 24" width="26" height="26">
        <circle cx="12" cy="12" r="9" fill="none" stroke="currentColor" strokeWidth="1.8" />
        <path d="M12 7v5l3 3" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
      </svg>
    ),
  },
  idle: {
    title: "Idle",
    description: "Auto-fade when inactive",
    icon: (
      <svg viewBox="0 0 24 24" width="26" height="26">
        <path d="M21 12.8A8.5 8.5 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8z" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" />
      </svg>
    ),
  },
};

const MODE_ORDER: Mode[] = [
  "static",
  "breathing",
  "strobing",
  "audio",
  "battery",
  "storm",
  "ripple",
  "radar",
  "morse",
  "disco",
  "schedule",
  "idle",
];

const EMPTY_STATUS: StatusInfo = {
  power: false,
  mode: "off",
  static_level: 2,
  interval_ms: 150,
  speed: 4,
  sensitivity: 60,
  min_level: 0,
  max_level: 2,
  waveform: 0,
  audio_beat: false,
  auto_dim_minutes: 0,
  critical_threshold: 10,
  day_start_hour: 7,
  night_start_hour: 22,
  idle_grace_s: 60,
  morse_text: "HI",
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
  const handleMinLevel = (v: number) => run("min", () => setMinLevel(v));
  const handleMaxLevel = (v: number) => run("max", () => setMaxLevel(v));
  const handleWaveform = (v: number) => run("wave", () => setWaveform(v));
  const handleAudioBeat = (on: boolean) => run("beat", () => setAudioBeat(on));
  const handleAutostart = (v: boolean) => run("auto", () => setAutostart(v));
  const handleTray = (v: boolean) => run("tray", () => setTrayClose(v));
  const handleAutoDim = (v: number) => run("dim", () => setAutoDim(v));
  const handleCritical = (v: number) => run("crit", () => setCriticalThreshold(v));
  const handleSchedule = (day: number, night: number) => run("schedule", () => setScheduleHours(day, night));
  const handleIdleGrace = (v: number) => run("idle", () => setIdleGrace(v));
  const handleMorse = (text: string) => run("morse", () => setMorseText(text));
  const handleQuit = () => quitApp();

  const battery = status.battery ?? emptyBattery();
  const controlsDisabled = !power || !status.driver_supported;

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
            <SpeedSlider value={status.speed} disabled={controlsDisabled} onChange={handleSpeed} />
            <AudioSens
              value={status.sensitivity}
              mode={status.mode}
              disabled={controlsDisabled}
              onChange={handleSens}
            />
            <ModeOptions
              mode={status.mode}
              minLevel={status.min_level}
              maxLevel={status.max_level}
              waveform={status.waveform}
              audioBeat={status.audio_beat}
              staticLevel={status.static_level}
              morseText={status.morse_text}
              disabled={controlsDisabled}
              onMinLevel={handleMinLevel}
              onMaxLevel={handleMaxLevel}
              onWaveform={handleWaveform}
              onAudioBeat={handleAudioBeat}
              onStaticLevel={handleStatic}
              onMorse={handleMorse}
            />
          </div>
          {status.audio_error && status.mode === "audio" && (
            <div className="install-result err">Audio: {status.audio_error}</div>
          )}
        </section>

        <SettingsPanel
          autostart={status.autostart}
          trayClose={status.tray_close}
          autoDim={status.auto_dim_minutes}
          critical={status.critical_threshold}
          dayHour={status.day_start_hour}
          nightHour={status.night_start_hour}
          idleGrace={status.idle_grace_s}
          disabled={busy.has("auto") || busy.has("tray")}
          onAutostart={handleAutostart}
          onTrayClose={handleTray}
          onAutoDim={handleAutoDim}
          onCritical={handleCritical}
          onSchedule={handleSchedule}
          onIdleGrace={handleIdleGrace}
          onQuit={handleQuit}
        />
      </main>

      <footer className="footer">
        GlowFX v0.2 · {status.os} · writes rate-limited to ≤10 Hz · battery {battery.percent}%
      </footer>
    </div>
  );
}