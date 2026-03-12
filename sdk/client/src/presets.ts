export enum Presets {
  SSS_1 = "SSS_1",
  SSS_2 = "SSS_2",
}

export interface PresetConfig {
  enablePermanentDelegate: boolean;
  enableTransferHook: boolean;
  defaultAccountFrozen: boolean;
}

export const PRESET_CONFIGS: Record<Presets, PresetConfig> = {
  [Presets.SSS_1]: {
    enablePermanentDelegate: false,
    enableTransferHook: false,
    defaultAccountFrozen: false,
  },
  [Presets.SSS_2]: {
    enablePermanentDelegate: true,
    enableTransferHook: true,
    defaultAccountFrozen: true,
  },
};

export const DEFAULT_MINTER_QUOTA = 1_000_000_000_000n;
