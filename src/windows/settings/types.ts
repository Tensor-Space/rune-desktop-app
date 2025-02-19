export interface Settings {
  shortcuts: ShortcutConfig;
  audio: AudioConfig;
  window: WindowConfig;
}

export interface ShortcutConfig {
  record_key: string;
  record_modifier: string;
}

export interface AudioConfig {
  default_device: string | null;
}

export interface WindowConfig {
  width: number;
  height: number;
}

export interface AudioDevice {
  name: string;
  id: string;
}
