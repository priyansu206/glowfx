import { useState, useEffect } from "react";
import { installUdev, type UdevResult } from "../api";

interface SettingsPanelProps {
  autostart: boolean;
  trayClose: boolean;
  disabled: boolean;
  onAutostart: (v: boolean) => void;
  onTrayClose: (v: boolean) => void;
  onQuit: () => void;
}

export default function SettingsPanel({
  autostart,
  trayClose,
  disabled,
  onAutostart,
  onTrayClose,
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