import { isTauri } from '@tauri-apps/api/core';

export * from './api';

export type TauriBridgeStatus = 'connected' | 'disconnected';
export type AppMode = 'desktop' | 'browser';

export interface TauriBridge {
  status: TauriBridgeStatus;
  mode: AppMode;
}

const isDesktop = isTauri();

export const tauriBridge: TauriBridge = {
  status: isDesktop ? 'connected' : 'disconnected',
  mode: isDesktop ? 'desktop' : 'browser',
};
