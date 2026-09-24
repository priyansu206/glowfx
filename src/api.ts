import { invoke } from "@tauri-apps/api/core";

export const MODES = [
  "static",
  "breathing",
  "strobing",
  "audio",
  "battery",
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

export function quitApp(): Promise<void> {
  return invoke("quit_app");
}

export function emptyBattery(): BatteryState {
  return { percent: 0, discharging: false, low: false };
}