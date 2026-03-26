import { tauriBridge } from './lib/tauri';
import {
  appendHistory as appendHistoryCommand,
  cancelChatStream as cancelChatStreamCommand,
  cancelDownload as cancelDownloadCommand,
  clearHistory as clearHistoryCommand,
  clearLlamaServerLogs,
  deletePreset as deletePresetCommand,
  getChatStreamStatuses,
  getDownloadStatuses,
  getHistory,
  getLlamaServerLogs,
  getLlamaServerStatus,
  getLlamaServerHealth,
  getPresets,
  getSettings,
  savePreset as savePresetCommand,
  saveSettings as saveSettingsCommand,
  subscribeToChatStreamEvent,
  startChatStream as startChatStreamCommand,
  startDownload as startDownloadCommand,
  startLlamaServer as startLlamaServerCommand,
  stopLlamaServer as stopLlamaServerCommand,
  type ChatStreamEvent,
  type ChatStreamStatus,
  type DownloadStatus,
  type HistoryEntry,
  type LlamaProcessStatus,
  type LlamaServerHealthStatus,
  type Preset,
  type Settings,
} from './lib/tauri/api';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

const LOG_LIMIT = 200;
const REFRESH_INTERVAL_MS = 4000;
const HISTORY_ROLES: HistoryEntry['role'][] = ['system', 'user', 'assistant'];

const createId = (prefix: string) =>
  `${prefix}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;

const formatTimestamp = (value: string) => {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString();
};

const formatStatusLabel = (value: string) =>
  value
    .replace(/_/g, ' ')
    .replace(/\b\w/g, (character) => character.toUpperCase());

const getToneForState = (value: string) => {
  switch (value.toLowerCase()) {
    case 'running':
    case 'streaming':
    case 'completed':
    case 'healthy':
      return 'success';
    case 'downloading':
      return 'info';
    case 'cancelled':
    case 'stopped':
    case 'idle':
    case 'not checked':
      return 'neutral';
    case 'failed':
    case 'unhealthy':
      return 'danger';
    default:
      return 'neutral';
  }
};

function App() {
  const [settings, setSettings] = useState<Settings>({
    serverUrl: 'http://127.0.0.1:8080',
    maxTokens: 512,
    temperature: 0.7,
  });
  const [processPath, setProcessPath] = useState('llama-server');
  const [processArgs, setProcessArgs] = useState('--port 8080');
  const [processStatus, setProcessStatus] = useState<LlamaProcessStatus>({
    running: false,
  });
  const [processHealth, setProcessHealth] =
    useState<LlamaServerHealthStatus | null>(null);
  const [refreshIssue, setRefreshIssue] = useState<string | null>(null);
  const [lastSuccessfulRefreshAt, setLastSuccessfulRefreshAt] = useState<
    string | null
  >(null);

  const [downloadUrl, setDownloadUrl] = useState('');
  const [downloadPath, setDownloadPath] = useState('');
  const [downloads, setDownloads] = useState<DownloadStatus[]>([]);
  const [llamaLogs, setLlamaLogs] = useState<string[]>([]);

  const [presets, setPresets] = useState<Preset[]>([]);
  const [presetDraft, setPresetDraft] = useState<Preset>({
    id: '',
    name: '',
    systemPrompt: '',
    createdAt: '',
  });

  const [historyEntries, setHistoryEntries] = useState<HistoryEntry[]>([]);
  const [historyRole, setHistoryRole] = useState<HistoryEntry['role']>('user');
  const [historyContent, setHistoryContent] = useState('');

  const [chatModel, setChatModel] = useState('default');
  const [chatPrompt, setChatPrompt] = useState('');
  const [chatLog, setChatLog] = useState<string[]>([]);
  const [chatStatuses, setChatStatuses] = useState<ChatStreamStatus[]>([]);

  const [message, setMessage] = useState('');
  const [busyActions, setBusyActions] = useState<Record<string, boolean>>({});
  const busyActionLocks = useRef(new Set<string>());

  const activeStream = useMemo(
    () => chatStatuses.find((item) => item.state === 'streaming'),
    [chatStatuses],
  );

  const isBusy = useCallback(
    (actionKey: string) => busyActions[actionKey] === true,
    [busyActions],
  );

  const setActionBusy = useCallback((actionKey: string, value: boolean) => {
    setBusyActions((prev) => {
      if (prev[actionKey] === value) {
        return prev;
      }

      return {
        ...prev,
        [actionKey]: value,
      };
    });
  }, []);

  const withBusyAction = useCallback(
    async (actionKey: string, task: () => Promise<void>): Promise<void> => {
      if (busyActionLocks.current.has(actionKey)) {
        return;
      }

      busyActionLocks.current.add(actionKey);
      setActionBusy(actionKey, true);

      try {
        await task();
      } finally {
        busyActionLocks.current.delete(actionKey);
        setActionBusy(actionKey, false);
      }
    },
    [setActionBusy],
  );

  const refreshAll = useCallback(async () => {
    await withBusyAction('refresh', async () => {
      try {
        const [
          loadedSettings,
          loadedProcess,
          loadedDownloads,
          loadedStreams,
          loadedLogs,
          loadedPresets,
          loadedHistory,
        ] = await Promise.all([
          getSettings(),
          getLlamaServerStatus(),
          getDownloadStatuses(),
          getChatStreamStatuses(),
          getLlamaServerLogs({ limit: LOG_LIMIT }),
          getPresets(),
          getHistory(),
        ]);
        setSettings(loadedSettings);
        setProcessStatus(loadedProcess);
        setDownloads(loadedDownloads);
        setChatStatuses(loadedStreams);
        setLlamaLogs(loadedLogs);
        setPresets(loadedPresets);
        setHistoryEntries(loadedHistory);
        setRefreshIssue(null);
        setLastSuccessfulRefreshAt(new Date().toISOString());
      } catch (error) {
        const errorMessage = String(error);
        setRefreshIssue(errorMessage);
        setMessage(errorMessage);
      }
    });
  }, [withBusyAction]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    void subscribeToChatStreamEvent((event: ChatStreamEvent) => {
      if (event.eventType === 'chunk' && event.data) {
        setChatLog((prev) => [...prev, event.data]);
      }
      if (event.eventType === 'error' && event.error) {
        setMessage(event.error);
      }
    }).then((dispose) => {
      unlisten = dispose;
    });

    void refreshAll();
    const intervalId = window.setInterval(() => {
      void refreshAll();
    }, REFRESH_INTERVAL_MS);

    return () => {
      window.clearInterval(intervalId);
      unlisten?.();
    };
  }, [refreshAll]);

  const saveSettings = async () => {
    await withBusyAction('saveSettings', async () => {
      try {
        await saveSettingsCommand(settings);
        setMessage('Settings saved.');
      } catch (error) {
        setMessage(String(error));
      }
    });
  };

  const resetPresetDraft = () => {
    setPresetDraft({
      id: '',
      name: '',
      systemPrompt: '',
      createdAt: '',
    });
  };

  const editPreset = (preset: Preset) => {
    setPresetDraft(preset);
  };

  const savePreset = async () => {
    await withBusyAction('savePreset', async () => {
      try {
        const trimmedName = presetDraft.name.trim();
        if (!trimmedName) {
          setMessage('Preset name is required.');
          return;
        }

        const payload: Preset = {
          id: presetDraft.id.trim() || createId('preset'),
          name: trimmedName,
          systemPrompt: presetDraft.systemPrompt,
          createdAt: presetDraft.createdAt.trim() || new Date().toISOString(),
        };

        const next = await savePresetCommand({
          preset: payload,
        });
        setPresets(next);
        setPresetDraft(payload);
        setMessage(`Preset '${payload.name}' saved.`);
      } catch (error) {
        setMessage(String(error));
      }
    });
  };

  const deletePreset = async (presetId: string) => {
    await withBusyAction(`deletePreset:${presetId}`, async () => {
      try {
        const next = await deletePresetCommand({
          presetId,
        });
        setPresets(next);
        if (presetDraft.id === presetId) {
          resetPresetDraft();
        }
        setMessage('Preset deleted.');
      } catch (error) {
        setMessage(String(error));
      }
    });
  };

  const appendHistoryEntry = async () => {
    await withBusyAction('appendHistory', async () => {
      try {
        const content = historyContent.trim();
        if (!content) {
          setMessage('History content is required.');
          return;
        }

        const entry: HistoryEntry = {
          id: createId('history'),
          role: historyRole,
          content,
          timestamp: new Date().toISOString(),
        };
        const next = await appendHistoryCommand({ entry });
        setHistoryEntries(next);
        setHistoryContent('');
        setMessage('History entry added.');
      } catch (error) {
        setMessage(String(error));
      }
    });
  };

  const clearHistory = async () => {
    await withBusyAction('clearHistory', async () => {
      try {
        await clearHistoryCommand();
        setHistoryEntries([]);
        setMessage('History cleared.');
      } catch (error) {
        setMessage(String(error));
      }
    });
  };

  const startProcess = async () => {
    await withBusyAction('startProcess', async () => {
      try {
        const args = processArgs
          .split(' ')
          .map((item) => item.trim())
          .filter(Boolean);
        const status = await startLlamaServerCommand({
          executablePath: processPath,
          args,
        });
        setProcessStatus(status);
        setProcessHealth(null);
        setMessage('llama-server started.');
      } catch (error) {
        setMessage(String(error));
      }
    });
  };

  const stopProcess = async () => {
    await withBusyAction('stopProcess', async () => {
      try {
        const status = await stopLlamaServerCommand();
        setProcessStatus(status);
        setProcessHealth(null);
        setMessage('llama-server stopped.');
      } catch (error) {
        setMessage(String(error));
      }
    });
  };

  const startDownload = async () => {
    await withBusyAction('startDownload', async () => {
      try {
        await startDownloadCommand({
          sourceUrl: downloadUrl,
          destinationPath: downloadPath,
        });
        const next = await getDownloadStatuses();
        setDownloads(next);
        setMessage('Download started.');
      } catch (error) {
        setMessage(String(error));
      }
    });
  };

  const cancelDownload = async (downloadId: string) => {
    await withBusyAction(`cancelDownload:${downloadId}`, async () => {
      try {
        await cancelDownloadCommand({ downloadId });
        const next = await getDownloadStatuses();
        setDownloads(next);
      } catch (error) {
        setMessage(String(error));
      }
    });
  };

  const startChat = async () => {
    await withBusyAction('startChat', async () => {
      try {
        setChatLog([]);
        await startChatStreamCommand({
          request: {
            serverUrl: settings.serverUrl,
            model: chatModel,
            messages: [{ role: 'user', content: chatPrompt }],
            maxTokens: settings.maxTokens,
            temperature: settings.temperature,
          },
        });
        const next = await getChatStreamStatuses();
        setChatStatuses(next);
        setMessage('Chat stream started.');
      } catch (error) {
        setMessage(String(error));
      }
    });
  };

  const cancelChat = async () => {
    if (!activeStream) return;
    await withBusyAction('cancelChat', async () => {
      try {
        await cancelChatStreamCommand({
          streamId: activeStream.streamId,
        });
        const next = await getChatStreamStatuses();
        setChatStatuses(next);
      } catch (error) {
        setMessage(String(error));
      }
    });
  };

  const clearLogs = async () => {
    await withBusyAction('clearLogs', async () => {
      try {
        await clearLlamaServerLogs();
        setLlamaLogs([]);
        setMessage('llama-server logs cleared.');
      } catch (error) {
        setMessage(String(error));
      }
    });
  };

  const checkProcessHealth = async () => {
    await withBusyAction('checkHealth', async () => {
      try {
        const health = await getLlamaServerHealth({
          url: `${settings.serverUrl.replace(/\/$/, '')}/health`,
        });
        setProcessHealth(health);
      } catch (error) {
        const healthUrl = `${settings.serverUrl.replace(/\/$/, '')}/health`;
        const errorMessage = String(error);
        setProcessHealth({
          healthy: false,
          statusCode: null,
          message: errorMessage,
          url: healthUrl,
        });
        setMessage(errorMessage);
      }
    });
  };

  const processHealthLabel = processHealth
    ? processHealth.healthy
      ? 'Healthy'
      : 'Unhealthy'
    : 'Not checked';
  const processHealthClass = processHealth
    ? processHealth.healthy
      ? 'healthy'
      : 'unhealthy'
    : 'idle';
  const processBadgeLabel = processStatus.running
    ? 'Running'
    : processStatus.lastExitCode !== undefined &&
        processStatus.lastExitCode !== null
      ? `Exited ${processStatus.lastExitCode}`
      : 'Stopped';
  const processBadgeTone = processStatus.running
    ? 'success'
    : processStatus.lastExitCode !== undefined &&
        processStatus.lastExitCode !== null
      ? 'danger'
      : 'neutral';
  const chatSummaryLabel = activeStream
    ? `Streaming ${activeStream.model}`
    : chatStatuses.length
      ? 'Idle'
      : 'No streams';
  const chatSummaryTone = activeStream
    ? 'info'
    : chatStatuses.some((item) => item.state === 'failed')
      ? 'danger'
        : chatStatuses.some((item) => item.state === 'completed')
        ? 'success'
        : 'neutral';
  const refreshStatusLabel = refreshIssue ? 'Refresh failed' : 'Sync live';
  const refreshStatusTone = refreshIssue ? 'danger' : 'success';
  const refreshStatusHint = refreshIssue
    ? refreshIssue
    : 'Auto-refresh polling is active.';
  const isRefreshing = isBusy('refresh');
  const isSavingSettings = isBusy('saveSettings');
  const isCheckingHealth = isBusy('checkHealth');
  const isProcessTransitionBusy =
    isBusy('startProcess') || isBusy('stopProcess');
  const isSavingPreset = isBusy('savePreset');
  const isAppendingHistory = isBusy('appendHistory');
  const isClearingHistory = isBusy('clearHistory');
  const isStartingDownload = isBusy('startDownload');
  const isStartingChat = isBusy('startChat');
  const isCancellingChat = isBusy('cancelChat');
  const isClearingLogs = isBusy('clearLogs');
  const isDeletingPreset = (presetId: string) => isBusy(`deletePreset:${presetId}`);
  const isCancellingDownload = (downloadId: string) =>
    isBusy(`cancelDownload:${downloadId}`);

  return (
    <main className="app-shell">
      <header className="page-header">
        <div>
          <p className="eyebrow">Migration item #n6</p>
          <h1>LlamaCppDesk Tauri control panel</h1>
          <div className="header-meta">
            <span className="hint">
              Last successful sync:{' '}
              {isRefreshing
                ? 'Refreshing...'
                : lastSuccessfulRefreshAt
                  ? formatTimestamp(lastSuccessfulRefreshAt)
                  : 'Waiting...'}
            </span>
          </div>
        </div>
        <button onClick={() => void refreshAll()} disabled={isRefreshing}>
          {isRefreshing ? 'Refreshing...' : 'Refresh'}
        </button>
      </header>

      <section className="grid">
        <article className="card">
          <h2>Settings</h2>
          <label>
            Server URL
            <input
              value={settings.serverUrl}
              onChange={(e) =>
                setSettings((prev) => ({ ...prev, serverUrl: e.target.value }))
              }
            />
          </label>
          <label>
            Max tokens
            <input
              type="number"
              value={settings.maxTokens}
              onChange={(e) =>
                setSettings((prev) => ({
                  ...prev,
                  maxTokens: Number(e.target.value),
                }))
              }
            />
          </label>
          <label>
            Temperature
            <input
              type="number"
              step="0.1"
              value={settings.temperature}
              onChange={(e) =>
                setSettings((prev) => ({
                  ...prev,
                  temperature: Number(e.target.value),
                }))
              }
            />
          </label>
          <button onClick={() => void saveSettings()} disabled={isSavingSettings}>
            {isSavingSettings ? 'Saving...' : 'Save settings'}
          </button>
        </article>

        <article className="card">
          <h2>llama-server</h2>
          <div className="status-summary">
            <span className={`status-badge ${refreshStatusTone}`}>
              {refreshStatusLabel}
            </span>
            <span className="hint">{refreshStatusHint}</span>
          </div>
          <div className="status-summary">
            <span className={`status-badge ${processBadgeTone}`}>
              {processBadgeLabel}
            </span>
            <span className="hint">PID {processStatus.pid ?? '-'}</span>
          </div>
          <div className="health-panel">
            <div className="panel-header">
              <h3>Health</h3>
              <button
                className="secondary"
                onClick={() => void checkProcessHealth()}
                disabled={isCheckingHealth}
              >
                {isCheckingHealth ? 'Checking...' : 'Check health'}
              </button>
            </div>
            <div className="health-summary">
              <span className={`status-pill ${processHealthClass}`}>
                {processHealthLabel}
              </span>
              <span className="hint">
                {processHealth?.statusCode !== undefined &&
                processHealth?.statusCode !== null
                  ? `HTTP ${processHealth.statusCode}`
                  : processHealth?.url ?? 'No health check run yet.'}
              </span>
            </div>
            {processHealth?.message ? (
              <p className="health-message">{processHealth.message}</p>
            ) : null}
          </div>
          <label>
            Executable path
            <input
              value={processPath}
              onChange={(e) => setProcessPath(e.target.value)}
            />
          </label>
          <label>
            Args
            <input
              value={processArgs}
              onChange={(e) => setProcessArgs(e.target.value)}
            />
          </label>
          <div className="row">
            <button onClick={() => void startProcess()} disabled={isProcessTransitionBusy}>
              {isBusy('startProcess') ? 'Starting...' : 'Start'}
            </button>
            <button onClick={() => void stopProcess()} disabled={isProcessTransitionBusy}>
              {isBusy('stopProcess') ? 'Stopping...' : 'Stop'}
            </button>
          </div>
          <div className="panel">
            <div className="panel-header">
              <h3>Logs</h3>
              <button onClick={() => void clearLogs()} disabled={isClearingLogs}>
                {isClearingLogs ? 'Clearing...' : 'Clear logs'}
              </button>
            </div>
            <pre className="log-panel">
              {llamaLogs.length ? llamaLogs.join('\n') : 'No llama-server logs yet.'}
            </pre>
          </div>
        </article>

        <article className="card">
          <div className="panel-header">
            <h2>Presets</h2>
            <button className="secondary" onClick={resetPresetDraft}>
              New preset
            </button>
          </div>
          <label>
            Name
            <input
              value={presetDraft.name}
              onChange={(e) =>
                setPresetDraft((prev) => ({ ...prev, name: e.target.value }))
              }
            />
          </label>
          <label>
            Preset ID
            <input
              value={presetDraft.id}
              onChange={(e) =>
                setPresetDraft((prev) => ({ ...prev, id: e.target.value }))
              }
              placeholder="Generated automatically for new presets"
            />
          </label>
          <label>
            System prompt
            <textarea
              value={presetDraft.systemPrompt}
              onChange={(e) =>
                setPresetDraft((prev) => ({
                  ...prev,
                  systemPrompt: e.target.value,
                }))
              }
              rows={4}
            />
          </label>
          <div className="row">
            <button onClick={() => void savePreset()} disabled={isSavingPreset}>
              {isSavingPreset ? 'Saving...' : 'Save preset'}
            </button>
            <button className="secondary" onClick={resetPresetDraft}>
              Clear form
            </button>
          </div>
          <div className="panel">
            <div className="panel-header">
              <h3>Saved presets</h3>
              <span className="hint">{presets.length} items</span>
            </div>
            <div className="entry-list">
              {presets.length ? (
                presets.map((preset) => (
                  <article className="entry-card" key={preset.id}>
                    <div className="entry-card-header">
                      <div>
                        <strong>{preset.name}</strong>
                        <div className="hint">{preset.id}</div>
                      </div>
                      <div className="hint">{formatTimestamp(preset.createdAt)}</div>
                    </div>
                    <p>{preset.systemPrompt || 'No system prompt stored.'}</p>
                    <div className="row">
                      <button
                        className="secondary"
                        onClick={() => editPreset(preset)}
                        disabled={isDeletingPreset(preset.id)}
                      >
                        Edit
                      </button>
                      <button
                        className="danger"
                        onClick={() => void deletePreset(preset.id)}
                        disabled={isDeletingPreset(preset.id)}
                      >
                        {isDeletingPreset(preset.id) ? 'Deleting...' : 'Delete'}
                      </button>
                    </div>
                  </article>
                ))
              ) : (
                <p className="empty-state">No presets saved yet.</p>
              )}
            </div>
          </div>
        </article>

        <article className="card">
          <div className="panel-header">
            <h2>History</h2>
            <button
              className="secondary"
              onClick={() => void clearHistory()}
              disabled={isClearingHistory}
            >
              {isClearingHistory ? 'Clearing...' : 'Clear history'}
            </button>
          </div>
          <div className="field-grid">
            <label>
              Role
              <select
                value={historyRole}
                onChange={(e) =>
                  setHistoryRole(e.target.value as HistoryEntry['role'])
                }
              >
                {HISTORY_ROLES.map((role) => (
                  <option key={role} value={role}>
                    {role}
                  </option>
                ))}
              </select>
            </label>
            <label>
              Count
              <input value={historyEntries.length} readOnly />
            </label>
          </div>
          <label>
            Content
            <textarea
              value={historyContent}
              onChange={(e) => setHistoryContent(e.target.value)}
              rows={4}
              placeholder="Append a history entry manually."
            />
          </label>
          <div className="row">
            <button onClick={() => void appendHistoryEntry()} disabled={isAppendingHistory}>
              {isAppendingHistory ? 'Appending...' : 'Append history'}
            </button>
          </div>
          <div className="panel">
            <div className="panel-header">
              <h3>Stored entries</h3>
              <span className="hint">Newest first</span>
            </div>
            <div className="entry-list">
              {historyEntries.length ? (
                [...historyEntries].reverse().map((entry) => (
                  <article className="entry-card" key={entry.id}>
                    <div className="entry-card-header">
                      <div>
                        <strong>{entry.role}</strong>
                        <div className="hint">{entry.id}</div>
                      </div>
                      <div className="hint">{formatTimestamp(entry.timestamp)}</div>
                    </div>
                    <p>{entry.content}</p>
                  </article>
                ))
              ) : (
                <p className="empty-state">No history entries saved yet.</p>
              )}
            </div>
          </div>
        </article>

        <article className="card">
          <h2>Downloader</h2>
          <div className="status-summary">
            <span className="status-badge neutral">
              {downloads.length ? `${downloads.length} tracked` : 'No downloads'}
            </span>
            <span className="hint">State updates refresh automatically</span>
          </div>
          <label>
            Source URL
            <input
              value={downloadUrl}
              onChange={(e) => setDownloadUrl(e.target.value)}
            />
          </label>
          <label>
            Destination path
            <input
              value={downloadPath}
              onChange={(e) => setDownloadPath(e.target.value)}
            />
          </label>
          <button onClick={() => void startDownload()} disabled={isStartingDownload}>
            {isStartingDownload ? 'Starting...' : 'Start download'}
          </button>
          <ul>
            {downloads.map((item) => (
              <li key={item.downloadId}>
                <div className="status-summary">
                  <span className={`status-badge ${getToneForState(item.state)}`}>
                    {formatStatusLabel(item.state)}
                  </span>
                  <strong>{item.downloadId}</strong>
                </div>
                {item.state === 'failed' && item.error ? (
                  <div className="hint">{item.error}</div>
                ) : null}
                {item.sourceUrl ? <div className="hint">{item.sourceUrl}</div> : null}
                {item.destinationPath ? (
                  <div className="hint">{item.destinationPath}</div>
                ) : null}
                <div className="hint">
                  {item.percentComplete !== null &&
                  item.percentComplete !== undefined
                    ? `${item.percentComplete.toFixed(1)}%`
                    : 'Progress unavailable'}
                </div>
                <div className="row">
                  <button
                    onClick={() => void cancelDownload(item.downloadId)}
                    disabled={isCancellingDownload(item.downloadId)}
                  >
                    {isCancellingDownload(item.downloadId)
                      ? 'Cancelling...'
                      : 'Cancel'}
                  </button>
                </div>
              </li>
            ))}
          </ul>
        </article>

        <article className="card chat-card">
          <h2>Chat stream</h2>
          <div className="status-summary">
            <span className={`status-badge ${chatSummaryTone}`}>
              {chatSummaryLabel}
            </span>
            <span className="hint">{chatStatuses.length} tracked</span>
          </div>
          <label>
            Model
            <input
              value={chatModel}
              onChange={(e) => setChatModel(e.target.value)}
            />
          </label>
          <label>
            Prompt
            <textarea
              value={chatPrompt}
              onChange={(e) => setChatPrompt(e.target.value)}
              rows={3}
            />
          </label>
          <div className="row">
            <button onClick={() => void startChat()} disabled={isStartingChat}>
              {isStartingChat ? 'Starting...' : 'Start stream'}
            </button>
            <button
              onClick={() => void cancelChat()}
              disabled={!activeStream || isCancellingChat}
            >
              {isCancellingChat ? 'Cancelling...' : 'Cancel stream'}
            </button>
          </div>
          <div className="panel">
            <div className="panel-header">
              <h3>Stream statuses</h3>
              <span className="hint">Live refresh updates</span>
            </div>
            <div className="entry-list">
              {chatStatuses.length ? (
                chatStatuses.map((status) => (
                  <article className="entry-card" key={status.streamId}>
                    <div className="entry-card-header">
                      <div className="status-summary">
                        <span
                          className={`status-badge ${getToneForState(status.state)}`}
                        >
                          {formatStatusLabel(status.state)}
                        </span>
                        <strong>{status.model}</strong>
                      </div>
                      <div className="hint">{status.streamId}</div>
                    </div>
                    <div className="hint">
                      Bytes received: {status.bytesReceived.toLocaleString()}
                    </div>
                    {status.error ? <p>{status.error}</p> : null}
                  </article>
                ))
              ) : (
                <p className="empty-state">No chat streams tracked yet.</p>
              )}
            </div>
          </div>
          <pre>{chatLog.join('')}</pre>
        </article>
      </section>

      <footer className="status-line">
        <span>Bridge: {tauriBridge.status}</span>
        <span>{message}</span>
      </footer>
    </main>
  );
}

export default App;
