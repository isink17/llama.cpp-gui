import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export type TauriBridgeStatus = 'connected';

export interface TauriBridge {
  status: TauriBridgeStatus;
  invokeCommand: <T>(command: string, payload?: object) => Promise<T>;
  listenToEvent: (
    event: string,
    handler: (payload: unknown) => void,
  ) => () => void;
}

export const tauriBridge: TauriBridge = {
  status: 'connected',
  invokeCommand: <T>(command: string, payload?: object) => invoke<T>(command, payload),
  listenToEvent: (event, handler) => {
    let unlisten: null | (() => void) = null;
    void listen(event, (eventPayload) => {
      handler(eventPayload.payload);
    }).then((dispose) => {
      unlisten = dispose;
    });

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  },
};
