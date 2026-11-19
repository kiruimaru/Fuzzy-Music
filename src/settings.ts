import { invoke } from "@tauri-apps/api/core";

export type DownloadQuality = "best" | "high" | "medium" | "standard" | "small" | "flac";
export type Theme = "dark" | "light";
export type AppSettings = { volume: number; downloadQuality?: DownloadQuality; musicFolder?: string; theme?: Theme };

// Issue #21 supplies persistence behind this interface.
export interface SettingsStore {
  getSettings(): Promise<AppSettings>;
  updateSettings(settings: Partial<AppSettings>): Promise<AppSettings>;
}

export function validVolume(volume: number): boolean {
  return Number.isFinite(volume) && volume >= 0 && volume <= 100;
}

export function createMemorySettingsStore(initial: AppSettings = { volume: 100 }): SettingsStore {
  let settings = { ...initial };
  return {
    async getSettings() { return { ...settings }; },
    async updateSettings(next) {
      if (next.volume !== undefined && !validVolume(next.volume)) throw { code: "invalid_input" };
      settings = { ...settings, ...next };
      return { ...settings };
    },
  };
}

export const settingsStore: SettingsStore = {
  getSettings: () => invoke<AppSettings>("get_settings"),
  updateSettings: settings => invoke<AppSettings>("update_settings", { settings }),
};
