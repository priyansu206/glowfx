interface SpeedSliderProps {
  value: number; // 1..=10
  disabled: boolean;
  onChange: (v: number) => void;
}

export const SPEED_LABELS: Record<number, string> = {
  1: "Calm",
  2: "Slow",
  3: "Slow",
  4: "Relaxed",
  5: "Moderate",
  6: "Moderate",
  7: "Brisk",
  8: "Fast",
  9: "Fast",
  10: "Max",
};

export default function SpeedSlider({ value, disabled, onChange }: SpeedSliderProps) {
  return (
    <label className={`control-row${disabled ? " disabled" : ""}`}>
      <span className="control-label">Speed</span>
      <input
        type="range"
        min={1}
        max={10}
        step={1}
        value={value}
        disabled={disabled}
        onChange={(e) => onChange(Number(e.target.value))}
      />
      <span className="control-value">{SPEED_LABELS[value] ?? value}</span>
    </label>
  );
}