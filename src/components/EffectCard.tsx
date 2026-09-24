import type { ReactNode } from "react";

interface EffectCardProps {
  mode: string;
  title: string;
  description: string;
  icon: ReactNode;
  selected: boolean;
  disabled: boolean;
  onClick: () => void;
}

export default function EffectCard({
  mode,
  title,
  description,
  icon,
  selected,
  disabled,
  onClick,
}: EffectCardProps) {
  return (
    <button
      type="button"
      className={`effect-card${selected ? " selected" : ""}${disabled ? " disabled" : ""}`}
      onClick={onClick}
      disabled={disabled}
      data-mode={mode}
      aria-pressed={selected}
    >
      <div className="effect-card-icon">{icon}</div>
      <div className="effect-card-body">
        <span className="effect-card-title">{title}</span>
        <span className="effect-card-desc">{description}</span>
      </div>
      <span className="effect-card-dot" aria-hidden="true" />
    </button>
  );
}