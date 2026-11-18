export type AppSettings = { volume: number };

// Issue #21 supplies persistence behind this interface.
export interface SettingsStore {
  getSettings(): Promise<AppSettings>;
  updateSettings(settings: AppSettings): Promise<AppSettings>;
}

export function validVolume(volume: number): boolean {
  return Number.isFinite(volume) && volume >= 0 && volume <= 100;
}

export function createMemorySettingsStore(initial: AppSettings = { volume: 100 }): SettingsStore {
  let settings = { ...initial };
  return {
    async getSettings() { return { ...settings }; },
    async updateSettings(next) {
      if (!validVolume(next.volume)) throw { code: "invalid_input" };
      settings = { ...next };
      return { ...settings };
    },
  };
}

export const settingsStore = createMemorySettingsStore();
