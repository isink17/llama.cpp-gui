export type TauriBridgeStatus = 'placeholder';

export interface TauriBridge {
  status: TauriBridgeStatus;
  invokeCommand: (command: string, payload?: unknown) => Promise<unknown>;
  listenToEvent: (event: string, handler: (payload: unknown) => void) => () => void;
}

const notReady = () => {
  throw new Error('Tauri bridge is a placeholder until migration wiring is added.');
};

export const tauriBridge: TauriBridge = {
  status: 'placeholder',
  invokeCommand: async () => notReady(),
  listenToEvent: () => () => undefined,
};
