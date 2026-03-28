import { isTauri } from '@tauri-apps/api/core';

export * from './api';

export type TauriBridgeStatus = 'connected' | 'disconnected';

export interface TauriBridge {
  status: TauriBridgeStatus;
}

export const tauriBridge: TauriBridge = {
  status: isTauri() ? 'connected' : 'disconnected',
};
