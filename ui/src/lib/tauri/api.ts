import { invoke } from '@tauri-apps/api/core';

export type Settings = {
  serverUrl: string;
  maxTokens: number;
  temperature: number;
};

export type LlamaProcessStatus = {
  running: boolean;
  pid?: number | null;
  lastExitCode?: number | null;
};

export type DownloadState = 'downloading' | 'completed' | 'cancelled' | 'failed';

export type DownloadStatus = {
  downloadId: string;
  sourceUrl: string;
  destinationPath: string;
  state: DownloadState;
  bytesDownloaded: number;
  totalBytes?: number | null;
  percentComplete?: number | null;
  error?: string | null;
};

export type ChatMessage = {
  role: string;
  content: string;
};

export type ChatStreamRequest = {
  serverUrl?: string | null;
  model: string;
  messages: ChatMessage[];
  maxTokens?: number | null;
  temperature?: number | null;
};

export type ChatStreamState = 'streaming' | 'completed' | 'cancelled' | 'failed';

export type ChatStreamStatus = {
  streamId: string;
  state: ChatStreamState;
  model: string;
  bytesReceived: number;
  error?: string | null;
};

export type ChatStreamEvent = {
  streamId: string;
  eventType: string;
  data?: string | null;
  state: ChatStreamState;
  error?: string | null;
};

export type SaveSettingsRequest = {
  settings: Settings;
};

export type StartLlamaServerRequest = {
  executablePath: string;
  args: string[];
};

export type GetLlamaServerLogsRequest = {
  limit?: number;
};

export type StartDownloadRequest = {
  sourceUrl: string;
  destinationPath: string;
};

export type CancelDownloadRequest = {
  downloadId: string;
};

export type StartChatStreamRequest = {
  request: ChatStreamRequest;
};

export type CancelChatStreamRequest = {
  streamId: string;
};

export const getSettings = () => invoke<Settings>('get_settings');

export const saveSettings = (settings: Settings) =>
  invoke<void>('save_settings', { settings } satisfies SaveSettingsRequest);

export const getLlamaServerStatus = () =>
  invoke<LlamaProcessStatus>('get_llama_server_status');

export const startLlamaServer = (request: StartLlamaServerRequest) =>
  invoke<LlamaProcessStatus>('start_llama_server', request);

export const stopLlamaServer = () =>
  invoke<LlamaProcessStatus>('stop_llama_server');

export const getLlamaServerLogs = (request: GetLlamaServerLogsRequest = {}) =>
  invoke<string[]>('get_llama_server_logs', request);

export const clearLlamaServerLogs = () =>
  invoke<void>('clear_llama_server_logs');

export const getDownloadStatuses = () =>
  invoke<DownloadStatus[]>('get_download_statuses');

export const startDownload = (request: StartDownloadRequest) =>
  invoke<DownloadStatus>('start_download', request);

export const cancelDownload = (request: CancelDownloadRequest) =>
  invoke<DownloadStatus>('cancel_download', request);

export const getChatStreamStatuses = () =>
  invoke<ChatStreamStatus[]>('get_chat_stream_statuses');

export const startChatStream = (request: StartChatStreamRequest) =>
  invoke<ChatStreamStatus>('start_chat_stream', request);

export const cancelChatStream = (request: CancelChatStreamRequest) =>
  invoke<ChatStreamStatus>('cancel_chat_stream', request);
