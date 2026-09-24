import { useState, useEffect } from "react";
import { installUdev, type UdevResult } from "../api";

interface SettingsPanelProps {
  autostart: boolean;
  trayClose: boolean;
  autoDim: number;
  critical: number;
  dayHour: number;
  nightHour: number;
  idleGrace: number;
  disabled: boolean;
  onAutostart: (v: boolean) => void;
  onTrayClose: (v: boolean) => void;
  onAutoDim: (v: number) => void;
  onCritical: (v: number) => void;
  onSchedule: (day: number, night: number) => void;
  onIdleGrace: (v: number) => void;
  onQuit: () => void;
}

function NumberField({
  label,
  hint,
  value,
  min,
  max,
  onChange,
}: {
  label: string;
  hint: string;
  value: number;
  min: number;
  max: number;
  onChange: (v: number) => void;
}) {
  return (
    <label className="toggle-row">
      <span className="toggle-row-text">
        <b>{label}</b>
        <small>{hint}</small>
      </span>
      <input
        className="num-input"
        type="number"
        min={min}
        max={max}
        value={value}
        onChange={(e) => {
          const n = Number(e.target.value);
          if (!Number.isNaN(n)) onChange(n);
        }}
      />
    </label>
  );
}

export default function SettingsPanel({
  autostart,
  trayClose,
  autoDim,
  critical,
  dayHour,
  nightHour,
  idleGrace,
  disabled,
  onAutostart,
  onTrayClose,
  onAutoDim,
  onCritical,
  onSchedule,
  onIdleGrace,
  onQuit,
}: SettingsPanelProps) {
  const [udev, setUdev] = useState<UdevResult | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    setUdev(null);
  }, []);

  async function handleInstallUdev() {
    setBusy(true);
    try {
      setUdev(await installUdev());
    } catch (e) {
      setUdev({ ok: false, message: String(e) });
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="settings">
      <div className="settings-title">Settings</div>
      <div className="settings-grid">
        <label className="toggle-row">
          <span className="toggle-row-text">
            <b>Run on system boot</b>
            <small>Launch GlowFX automatically after sign-in</small>
          </span>
          <span className="mini-toggle">
            <input
              type="checkbox"
              checked={autostart}
              disabled={disabled}
              onChange={(e) => onAutostart(e.target.checked)}
            />
            <span className="mini-toggle-ui" />
          </span>
        </label>

        <label className="toggle-row">
          <span className="toggle-row-text">
            <b>Close to system tray</b>
            <small>Keep running in the tray when the window closes</small>
          </span>
          <span className="mini-toggle">
            <input
              type="checkbox"
              checked={trayClose}
              onChange={(e) => onTrayClose(e.target.checked)}
            />
            <span className="mini-toggle-ui" />
          </span>
        </label>

        <div className="toggle-row">
          <span className="toggle-row-text">
            <b>Linux permissions (udev)</b>
            <small>Allow backlight writes without sudo (Linux only)</small>
          </span>
          <button
            type="button"
            className="btn-primary"
            onClick={handleInstallUdev}
            disabled={disabled || busy}
          >
            {busy ? "Working…" : "Install"}
          </button>
        </div>

        <NumberField
          label="Auto-dim timeout"
          hint="Minutes before the backlight fades to off (0 = never)"
          value={autoDim}
          min={0}
          max={480}
          onChange={onAutoDim}
        />

        <NumberField
          label="Critical battery %"
          hint="At or below this (while discharging) the backlight SOS-blinks"
          value={critical}
          min={1}
          max={50}
          onChange={onCritical}
        />

        <div className="toggle-row">
          <span className="toggle-row-text">
            <b>Schedule hours</b>
            <small>
              Bright from {dayHour}:00 to {nightHour}:00; dim outside
            </small>
          </span>
          <div className="schedule-pair">
            <input
              className="num-input"
              type="number"
              min={0}
              max={23}
              value={dayHour}
              aria-label="bright start hour"
              onChange={(e) => onSchedule(Number(e.target.value) || 0, nightHour)}
            />
            <span className="schedule-sep">→</span>
            <input
              className="num-input"
              type="number"
              min={0}
              max={23}
              value={nightHour}
              aria-label="dim start hour"
              onChange={(e) => onSchedule(dayHour, Number(e.target.value) || 0)}
            />
          </div>
        </div>

        <NumberField
          label="Idle grace"
          hint="Seconds of inactivity before the Idle effect fades out"
          value={idleGrace}
          min={15}
          max={3600}
          onChange={onIdleGrace}
        />
      </div>

      {udev && (
        <div className={`install-result${udev.ok ? " ok" : " err"}`}>{udev.message}</div>
      )}

      <button type="button" className="btn-quit" onClick={onQuit}>
        Quit GlowFX
      </button>
    </section>
  );
}