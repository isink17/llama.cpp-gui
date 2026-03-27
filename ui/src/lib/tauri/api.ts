import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export type Settings = {
  llamaServerPath: string;
  modelPath: string;
  host: string;
  port: number;
  contextSize: number;
  threads: number;
  gpuLayers: number;
  temperature: number;
  maxTokens: number;
  downloadFolder: string;
  recentModelPaths: string[];
  recentServerPaths: string[];
  recentModelUrls: string[];
};

export type LlamaProcessStatus = {
  running: boolean;
  pid?: number | null;
  lastExitCode?: number | null;
};

export type LlamaServerHealthStatus = {
  healthy: boolean;
  statusCode?: number | null;
  message?: string | null;
  url: string;
};

export type Preset = {
  id: string;
  name: string;
  modelPath: string;
  host: string;
  port: number;
  contextSize: number;
  threads: number;
  gpuLayers: number;
  temperature: number;
  maxTokens: number;
};

export type HistoryRole = 'system' | 'user' | 'assistant';

export type HistoryEntry = {
  id: string;
  role: HistoryRole;
  content: string;
  timestamp: string;
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

export type SavePresetRequest = {
  preset: Preset;
};

export type SavePresetResponse = Preset[];

export type DeletePresetRequest = {
  presetId: string;
};

export type DeletePresetResponse = Preset[];

export type AppendHistoryRequest = {
  entry: HistoryEntry;
};

export type AppendHistoryResponse = HistoryEntry[];

export type ClearHistoryResponse = void;

export type StartLlamaServerRequest = {
  executablePath: string;
  args: string[];
};

export type GetLlamaServerLogsRequest = {
  limit?: number;
};

export type CheckLlamaServerHealthRequest = {
  url: string;
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

export const getPresets = () => invoke<Preset[]>('get_presets');

export const savePreset = (request: SavePresetRequest) =>
  invoke<SavePresetResponse>('save_preset', request);

export const deletePreset = (request: DeletePresetRequest) =>
  invoke<DeletePresetResponse>('delete_preset', request);

export const getHistory = () => invoke<HistoryEntry[]>('get_history');

export const appendHistory = (request: AppendHistoryRequest) =>
  invoke<AppendHistoryResponse>('append_history', request);

export const clearHistory = () => invoke<ClearHistoryResponse>('clear_history');

export const subscribeToChatStreamEvent = (
  handler: (event: ChatStreamEvent) => void,
): Promise<UnlistenFn> =>
  listen<ChatStreamEvent>('chat_stream_event', (event) => {
    handler(event.payload);
  });

export const onChatStreamEvent = subscribeToChatStreamEvent;

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

export const checkLlamaServerHealth = (request: CheckLlamaServerHealthRequest) =>
  invoke<LlamaServerHealthStatus>('check_llama_server_health', request);

export const getLlamaServerHealth = checkLlamaServerHealth;

export type WaitForServerReadyRequest = {
  serverUrl: string;
  timeoutSecs?: number | null;
};

export const waitForServerReady = (request: WaitForServerReadyRequest) =>
  invoke<LlamaServerHealthStatus>('wait_for_server_ready', request);

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

export type ResolvedModelDownload = {
  downloadUrl: string;
  suggestedFileName: string;
  requestHeaders?: Record<string, string> | null;
};

export type ResolveModelReferenceRequest = {
  source: string;
  input: string;
  hfToken?: string | null;
};

export type ListHuggingFaceFilesRequest = {
  input: string;
  hfToken?: string | null;
};

export type ListOllamaTagsRequest = {
  input: string;
};

export const resolveModelReference = (request: ResolveModelReferenceRequest) =>
  invoke<ResolvedModelDownload>('resolve_model_reference', request);

export const listHuggingFaceFiles = (request: ListHuggingFaceFilesRequest) =>
  invoke<string[]>('list_hugging_face_files', request);

export const listOllamaTags = (request: ListOllamaTagsRequest) =>
  invoke<string[]>('list_ollama_tags', request);

export type PickFileRequest = {
  title: string;
  extensions: string[];
};

export type PickFolderRequest = {
  title: string;
};

export const pickFile = (request: PickFileRequest) =>
  invoke<string | null>('pick_file', request);

export const pickFolder = (request: PickFolderRequest) =>
  invoke<string | null>('pick_folder', request);
