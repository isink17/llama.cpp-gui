export * from './api';

export type TauriBridgeStatus = 'connected';

export interface TauriBridge {
  status: TauriBridgeStatus;
}

export const tauriBridge: TauriBridge = {
  status: 'connected',
};
