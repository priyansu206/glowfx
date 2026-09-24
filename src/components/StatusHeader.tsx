import type { BatteryState } from "../api";

interface StatusHeaderProps {
  power: boolean;
  driverSupported: boolean;
  driverPath: string;
  battery: BatteryState | null;
  mode: string;
  onPower: () => void;
}

export default function StatusHeader({
  power,
  driverSupported,
  driverPath,
  battery,
  mode,
  onPower,
}: StatusHeaderProps) {
  const chipClass = !driverSupported
    ? "chip chip-error"
    : power
      ? "chip chip-on"
      : "chip chip-off";

  const chipLabel = !driverSupported
    ? "Driver unavailable"
    : power
      ? "Backlight ON"
      : "Backlight OFF";

  return (
    <header className="status-header">
      <div className="brand">
        <span className="brand-logo" aria-hidden="true">
          <svg viewBox="0 0 32 32" width="30" height="30">
            <rect x="2" y="2" width="28" height="28" rx="7" fill="#10141b" />
            <path
              d="M8 21.5V10.5l16 11V10.5"
              fill="none"
              stroke="#a9c7ff"
              strokeWidth="2.6"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
            <circle cx="16" cy="16" r="2.4" fill="#a9c7ff" />
          </svg>
        </span>
        <div className="brand-text">
          <h1>GlowFX</h1>
          <p>Lenovo keyboard backlight suite</p>
        </div>
      </div>

      <div className="status-meta">
        {battery && (
          <span className={`chip${battery.low ? " chip-warn" : ""}`} title={`Battery ${battery.percent}%`}>
            {battery.discharging ? "Discharge" : "Charging"} {battery.percent}%
            {battery.low ? " · LOW" : ""}
          </span>
        )}
        <span className="chip chip-neutral" title={driverPath}>
          {driverSupported ? driverPath.split("/").pop() : "no hw"}
        </span>
        <span className="chip chip-mode">{mode}</span>
        <span className={chipClass}>{chipLabel}</span>
      </div>

      <button
        type="button"
        className={`power-toggle${power ? " on" : ""}`}
        onClick={onPower}
        aria-pressed={power}
      >
        <span className="power-toggle-track">
          <span className="power-toggle-thumb" />
        </span>
        <span className="power-toggle-label">{power ? "Power On" : "Power Off"}</span>
      </button>
    </header>
  );
}