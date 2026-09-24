import { invoke } from "@tauri-apps/api/core";

export const MODES = [
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
] as const;

export type Mode = (typeof MODES)[number];

export interface BatteryState {
  percent: number;
  discharging: boolean;
  low: boolean;
}

export interface StatusInfo {
  power: boolean;
  mode: string;
  static_level: number;
  interval_ms: number;
  speed: number;
  sensitivity: number;
  min_level: number;
  max_level: number;
  waveform: number;
  audio_beat: boolean;
  auto_dim_minutes: number;
  critical_threshold: number;
  day_start_hour: number;
  night_start_hour: number;
  idle_grace_s: number;
  morse_text: string;
  driver_supported: boolean;
  driver_path: string;
  driver_error: string | null;
  audio_error: string | null;
  battery: BatteryState | null;
  autostart: boolean;
  tray_close: boolean;
  os: string;
}

export interface UdevResult {
  ok: boolean;
  message: string;
}

export function setPower(on: boolean): Promise<void> {
  return invoke("set_power", { on });
}

export function setMode(mode: Mode): Promise<void> {
  return invoke("set_mode", { mode });
}

export function setSpeed(speed: number): Promise<void> {
  return invoke("set_speed", { speed });
}

export function setSensitivity(sensitivity: number): Promise<void> {
  return invoke("set_sensitivity", { sensitivity });
}

export function setStaticLevel(level: number): Promise<void> {
  return invoke("set_static_level", { level });
}

export function getStatus(): Promise<StatusInfo> {
  return invoke("get_status");
}

export function installUdev(): Promise<UdevResult> {
  return invoke("install_udev");
}

export function setAutostart(enabled: boolean): Promise<void> {
  return invoke("set_autostart", { enabled });
}

export function setTrayClose(enabled: boolean): Promise<void> {
  return invoke("set_tray_close", { enabled });
}

export function setMinLevel(level: number): Promise<void> {
  return invoke("set_min_level", { level });
}

export function setMaxLevel(level: number): Promise<void> {
  return invoke("set_max_level", { level });
}

export function setWaveform(waveform: number): Promise<void> {
  return invoke("set_waveform", { waveform });
}

export function setAudioBeat(on: boolean): Promise<void> {
  return invoke("set_audio_beat", { on });
}

export function setAutoDim(minutes: number): Promise<void> {
  return invoke("set_auto_dim", { minutes });
}

export function setCriticalThreshold(threshold: number): Promise<void> {
  return invoke("set_critical_threshold", { threshold });
}

export function setScheduleHours(day: number, night: number): Promise<void> {
  return invoke("set_schedule_hours", { day, night });
}

export function setIdleGrace(seconds: number): Promise<void> {
  return invoke("set_idle_grace", { seconds });
}

export function setMorseText(text: string): Promise<void> {
  return invoke("set_morse_text", { text });
}

export function quitApp(): Promise<void> {
  return invoke("quit_app");
}

export function emptyBattery(): BatteryState {
  return { percent: 0, discharging: false, low: false };
}