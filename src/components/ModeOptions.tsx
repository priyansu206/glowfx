interface ModeOptionsProps {
  mode: string;
  minLevel: number;
  maxLevel: number;
  waveform: number;
  audioBeat: boolean;
  staticLevel: number;
  morseText: string;
  disabled: boolean;
  onMinLevel: (v: number) => void;
  onMaxLevel: (v: number) => void;
  onWaveform: (v: number) => void;
  onAudioBeat: (on: boolean) => void;
  onStaticLevel: (v: number) => void;
  onMorse: (text: string) => void;
}

const LEVEL_NAMES = ["Off", "Low", "High"];

const MIN_MAX_MODES = ["breathing", "strobing", "storm", "ripple", "radar", "disco", "schedule", "idle"];
const WAVE_MODES = ["strobing"];

function LevelPicker({
  label,
  value,
  disabled,
  onChange,
}: {
  label: string;
  value: number;
  disabled: boolean;
  onChange: (v: number) => void;
}) {
  return (
    <div className="control-row">
      <span className="control-label">{label}</span>
      <div className="level-picker">
        {[0, 1, 2].map((l) => (
          <button
            key={l}
            type="button"
            className={`level-btn${value === l ? " active" : ""}`}
            disabled={disabled}
            onClick={() => onChange(l)}
          >
            {LEVEL_NAMES[l]}
          </button>
        ))}
      </div>
    </div>
  );
}

export default function ModeOptions({
  mode,
  minLevel,
  maxLevel,
  waveform,
  audioBeat,
  staticLevel,
  morseText,
  disabled,
  onMinLevel,
  onMaxLevel,
  onWaveform,
  onAudioBeat,
  onStaticLevel,
  onMorse,
}: ModeOptionsProps) {
  const showRange = MIN_MAX_MODES.includes(mode);
  const showWave = WAVE_MODES.includes(mode);

  return (
    <>
      {mode === "static" && (
        <LevelPicker label="Static level" value={staticLevel} disabled={disabled} onChange={onStaticLevel} />
      )}
      {showRange && (
        <>
          <LevelPicker label="Min level" value={minLevel} disabled={disabled} onChange={onMinLevel} />
          <LevelPicker label="Max level" value={maxLevel} disabled={disabled} onChange={onMaxLevel} />
        </>
      )}
      {showWave && (
        <div className="control-row">
          <span className="control-label">Waveform</span>
          <div className="level-picker">
            <button type="button" className={`level-btn${waveform === 0 ? " active" : ""}`} disabled={disabled} onClick={() => onWaveform(0)}>
              Square
            </button>
            <button type="button" className={`level-btn${waveform === 1 ? " active" : ""}`} disabled={disabled} onClick={() => onWaveform(1)}>
              Decay
            </button>
          </div>
        </div>
      )}
      {mode === "audio" && (
        <div className="control-row">
          <span className="control-label">Reactivity</span>
          <div className="level-picker">
            <button type="button" className={`level-btn${!audioBeat ? " active" : ""}`} disabled={disabled} onClick={() => onAudioBeat(false)}>
              Level
            </button>
            <button type="button" className={`level-btn${audioBeat ? " active" : ""}`} disabled={disabled} onClick={() => onAudioBeat(true)}>
              Beat sync
            </button>
          </div>
        </div>
      )}
      {mode === "morse" && (
        <div className="control-row">
          <span className="control-label">Morse</span>
          <input
            type="text"
            className="morse-input"
            value={morseText}
            maxLength={32}
            disabled={disabled}
            onChange={(e) => onMorse(e.target.value)}
          />
        </div>
      )}
    </>
  );
}