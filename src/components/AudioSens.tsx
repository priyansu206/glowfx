interface AudioSensProps {
  value: number; // 0..=100
  mode: string;
  disabled: boolean;
  onChange: (v: number) => void;
}

export default function AudioSens({ value, mode, disabled, onChange }: AudioSensProps) {
  const active = mode === "audio";
  return (
    <label className={`control-row${disabled || !active ? " disabled" : ""}`}>
      <span className="control-label">Audio sensitivity</span>
      <input
        type="range"
        min={0}
        max={100}
        step={1}
        value={value}
        disabled={disabled || !active}
        onChange={(e) => onChange(Number(e.target.value))}
      />
      <span className="control-value">{value}%</span>
    </label>
  );
}