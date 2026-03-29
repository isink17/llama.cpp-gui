import { invoke, isTauri, type InvokeArgs } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  clearRemoteApiConfig,
  getRemoteApiConfig,
  getRemoteApiDefaults,
  hasRemoteApiConfig,
  remoteRequest,
  subscribeToRemoteChatEvents,
  setRemoteApiConfig,
  type RemoteApiConfig,
} from '../remote/client';

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
  requestHeaders?: Record<string, string> | null;
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

const TAURI_RUNTIME_ERROR =
  'Tauri runtime is not available. Launch the app through the Tauri shell instead of the Vite dev server.';

const ensureTauriRuntime = () => {
  if (!isTauri()) {
    throw new Error(TAURI_RUNTIME_ERROR);
  }
};

const invokeTauri = <T>(command: string, args?: InvokeArgs) => {
  ensureTauriRuntime();
  return invoke<T>(command, args);
};

const browserUnsupported = (feature: string) => {
  throw new Error(`${feature} is only available in the desktop runtime.`);
};

export {
  clearRemoteApiConfig,
  getRemoteApiConfig,
  getRemoteApiDefaults,
  hasRemoteApiConfig,
  setRemoteApiConfig,
};
export type { RemoteApiConfig };

export const getSettings = () =>
  isTauri() ? invokeTauri<Settings>('get_settings') : remoteRequest<Settings>('/api/settings');

export const saveSettings = (settings: Settings) =>
  isTauri()
    ? invokeTauri<void>('save_settings', { settings } satisfies SaveSettingsRequest)
    : remoteRequest<Settings>('/api/settings', {
        method: 'PUT',
        body: JSON.stringify(settings),
      }).then(() => {});

export const getPresets = () =>
  isTauri() ? invokeTauri<Preset[]>('get_presets') : remoteRequest<Preset[]>('/api/presets');

export const savePreset = (request: SavePresetRequest) =>
  isTauri()
    ? invokeTauri<SavePresetResponse>('save_preset', request)
    : remoteRequest<SavePresetResponse>('/api/presets', {
        method: 'POST',
        body: JSON.stringify(request.preset),
      });

export const deletePreset = (request: DeletePresetRequest) =>
  isTauri()
    ? invokeTauri<DeletePresetResponse>('delete_preset', request)
    : remoteRequest<DeletePresetResponse>(`/api/presets/${encodeURIComponent(request.presetId)}`, {
        method: 'DELETE',
      });

export const getHistory = () =>
  isTauri() ? invokeTauri<HistoryEntry[]>('get_history') : remoteRequest<HistoryEntry[]>('/api/history');

export const appendHistory = (request: AppendHistoryRequest) =>
  isTauri()
    ? invokeTauri<AppendHistoryResponse>('append_history', request)
    : remoteRequest<AppendHistoryResponse>('/api/history', {
        method: 'POST',
        body: JSON.stringify(request.entry),
      });

export const clearHistory = () =>
  isTauri()
    ? invokeTauri<ClearHistoryResponse>('clear_history')
    : remoteRequest<void>('/api/history', {
        method: 'DELETE',
      });

export const subscribeToChatStreamEvent = (
  handler: (event: ChatStreamEvent) => void,
): Promise<UnlistenFn> =>
  isTauri()
    ? listen<ChatStreamEvent>('chat_stream_event', (event) => {
        handler(event.payload);
      })
    : subscribeToRemoteChatEvents((event) => {
        handler(event.payload as ChatStreamEvent);
      });

export const onChatStreamEvent = subscribeToChatStreamEvent;

export const getLlamaServerStatus = () =>
  isTauri()
    ? invokeTauri<LlamaProcessStatus>('get_llama_server_status')
    : remoteRequest<LlamaProcessStatus>('/api/process/status');

export const startLlamaServer = (request: StartLlamaServerRequest) =>
  isTauri()
    ? invokeTauri<LlamaProcessStatus>('start_llama_server', request)
    : remoteRequest<LlamaProcessStatus>('/api/process/start', {
        method: 'POST',
        body: JSON.stringify(request),
      });

export const stopLlamaServer = () =>
  isTauri()
    ? invokeTauri<LlamaProcessStatus>('stop_llama_server')
    : remoteRequest<LlamaProcessStatus>('/api/process/stop', {
        method: 'POST',
      });

export const getLlamaServerLogs = (request: GetLlamaServerLogsRequest = {}) =>
  isTauri()
    ? invokeTauri<string[]>('get_llama_server_logs', request)
    : remoteRequest<string[]>(
        `/api/process/logs${request.limit !== undefined ? `?limit=${encodeURIComponent(String(request.limit))}` : ''}`,
      );

export const clearLlamaServerLogs = () =>
  isTauri()
    ? invokeTauri<void>('clear_llama_server_logs')
    : remoteRequest<void>('/api/process/logs', {
        method: 'DELETE',
      });

export const checkLlamaServerHealth = (request: CheckLlamaServerHealthRequest) =>
  isTauri()
    ? invokeTauri<LlamaServerHealthStatus>('check_llama_server_health', request)
    : remoteRequest<LlamaServerHealthStatus>('/api/process/health', {
        method: 'POST',
        body: JSON.stringify({ url: request.url }),
      });

export const getLlamaServerHealth = checkLlamaServerHealth;

export type WaitForServerReadyRequest = {
  serverUrl: string;
  timeoutSecs?: number | null;
};

export const waitForServerReady = (request: WaitForServerReadyRequest) =>
  isTauri()
    ? invokeTauri<LlamaServerHealthStatus>('wait_for_server_ready', request)
    : remoteRequest<LlamaServerHealthStatus>('/api/process/wait-ready', {
        method: 'POST',
        body: JSON.stringify({
          serverUrl: request.serverUrl,
          timeoutSecs: request.timeoutSecs ?? null,
        }),
      });

export const getDownloadStatuses = () =>
  isTauri()
    ? invokeTauri<DownloadStatus[]>('get_download_statuses')
    : remoteRequest<DownloadStatus[]>('/api/downloads');

export const startDownload = (request: StartDownloadRequest) =>
  isTauri()
    ? invokeTauri<DownloadStatus>('start_download', request)
    : remoteRequest<DownloadStatus>('/api/downloads', {
        method: 'POST',
        body: JSON.stringify(request),
      });

export const cancelDownload = (request: CancelDownloadRequest) =>
  isTauri()
    ? invokeTauri<DownloadStatus>('cancel_download', request)
    : remoteRequest<DownloadStatus>(`/api/downloads/${encodeURIComponent(request.downloadId)}`, {
        method: 'DELETE',
      });

export const getChatStreamStatuses = () =>
  isTauri()
    ? invokeTauri<ChatStreamStatus[]>('get_chat_stream_statuses')
    : remoteRequest<ChatStreamStatus[]>('/api/chat/statuses');

export const startChatStream = (request: StartChatStreamRequest) =>
  isTauri()
    ? invokeTauri<ChatStreamStatus>('start_chat_stream', request)
    : remoteRequest<ChatStreamStatus>('/api/chat/start', {
        method: 'POST',
        body: JSON.stringify(request),
      });

export const cancelChatStream = (request: CancelChatStreamRequest) =>
  isTauri()
    ? invokeTauri<ChatStreamStatus>('cancel_chat_stream', request)
    : remoteRequest<ChatStreamStatus>('/api/chat/cancel', {
        method: 'POST',
        body: JSON.stringify(request),
      });

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
  isTauri()
    ? invokeTauri<ResolvedModelDownload>('resolve_model_reference', request)
    : browserUnsupported('Model resolution');

export const listHuggingFaceFiles = (request: ListHuggingFaceFilesRequest) =>
  isTauri()
    ? invokeTauri<string[]>('list_hugging_face_files', request)
    : browserUnsupported('Model resolution');

export const listOllamaTags = (request: ListOllamaTagsRequest) =>
  isTauri()
    ? invokeTauri<string[]>('list_ollama_tags', request)
    : browserUnsupported('Model resolution');

export type PickFileRequest = {
  title: string;
  extensions: string[];
};

export type PickFolderRequest = {
  title: string;
};

export const pickFile = (request: PickFileRequest) =>
  isTauri() ? invokeTauri<string | null>('pick_file', request) : browserUnsupported('File picker');

export const pickFolder = (request: PickFolderRequest) =>
  isTauri() ? invokeTauri<string | null>('pick_folder', request) : browserUnsupported('Folder picker');
