import { tauriBridge } from './lib/tauri';
import {
  cancelChatStream as cancelChatStreamCommand,
  cancelDownload as cancelDownloadCommand,
  clearLlamaServerLogs,
  getChatStreamStatuses,
  getDownloadStatuses,
  getLlamaServerLogs,
  getLlamaServerStatus,
  getSettings,
  saveSettings as saveSettingsCommand,
  startChatStream as startChatStreamCommand,
  startDownload as startDownloadCommand,
  startLlamaServer as startLlamaServerCommand,
  stopLlamaServer as stopLlamaServerCommand,
  type ChatStreamEvent,
  type ChatStreamStatus,
  type DownloadStatus,
  type LlamaProcessStatus,
  type Settings,
} from './lib/tauri/api';
import { useCallback, useEffect, useMemo, useState } from 'react';

type LlamaServerHealthStatus = {
  healthy: boolean;
  statusCode?: number | null;
  message?: string | null;
  url: string;
};

type Preset = {
  id: string;
  name: string;
  systemPrompt: string;
  createdAt: string;
};

type HistoryEntry = {
  id: string;
  role: 'system' | 'user' | 'assistant';
  content: string;
  timestamp: string;
};

const LOG_LIMIT = 200;
const REFRESH_INTERVAL_MS = 4000;
const HISTORY_ROLES: HistoryEntry['role'][] = ['system', 'user', 'assistant'];

const createId = (prefix: string) =>
  `${prefix}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;

const formatTimestamp = (value: string) => {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString();
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
  const [processHealthBusy, setProcessHealthBusy] = useState(false);

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

  const activeStream = useMemo(
    () => chatStatuses.find((item) => item.state === 'streaming'),
    [chatStatuses],
  );

  const refreshAll = useCallback(async () => {
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
        tauriBridge.invokeCommand<Preset[]>('get_presets'),
        tauriBridge.invokeCommand<HistoryEntry[]>('get_history'),
      ]);
      setSettings(loadedSettings);
      setProcessStatus(loadedProcess);
      setDownloads(loadedDownloads);
      setChatStatuses(loadedStreams);
      setLlamaLogs(loadedLogs);
      setPresets(loadedPresets);
      setHistoryEntries(loadedHistory);
    } catch (error) {
      setMessage(String(error));
    }
  }, []);

  useEffect(() => {
    const stopListening = tauriBridge.listenToEvent(
      'chat_stream_event',
      (payload) => {
        const event = payload as ChatStreamEvent;
        if (event.eventType === 'chunk' && event.data) {
          setChatLog((prev) => [...prev, event.data as string]);
        }
        if (event.eventType === 'error' && event.error) {
          setMessage(event.error);
        }
      },
    );

    void refreshAll();
    const intervalId = window.setInterval(() => {
      void refreshAll();
    }, REFRESH_INTERVAL_MS);

    return () => {
      window.clearInterval(intervalId);
      stopListening();
    };
  }, [refreshAll]);

  const saveSettings = async () => {
    try {
      await saveSettingsCommand(settings);
      setMessage('Settings saved.');
    } catch (error) {
      setMessage(String(error));
    }
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

      const next = await tauriBridge.invokeCommand<Preset[]>('save_preset', {
        preset: payload,
      });
      setPresets(next);
      setPresetDraft(payload);
      setMessage(`Preset '${payload.name}' saved.`);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const deletePreset = async (presetId: string) => {
    try {
      const next = await tauriBridge.invokeCommand<Preset[]>('delete_preset', {
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
  };

  const appendHistoryEntry = async () => {
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
      const next = await tauriBridge.invokeCommand<HistoryEntry[]>(
        'append_history',
        { entry },
      );
      setHistoryEntries(next);
      setHistoryContent('');
      setMessage('History entry added.');
    } catch (error) {
      setMessage(String(error));
    }
  };

  const clearHistory = async () => {
    try {
      await tauriBridge.invokeCommand('clear_history');
      setHistoryEntries([]);
      setMessage('History cleared.');
    } catch (error) {
      setMessage(String(error));
    }
  };

  const startProcess = async () => {
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
  };

  const stopProcess = async () => {
    try {
      const status = await stopLlamaServerCommand();
      setProcessStatus(status);
      setProcessHealth(null);
      setMessage('llama-server stopped.');
    } catch (error) {
      setMessage(String(error));
    }
  };

  const startDownload = async () => {
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
  };

  const cancelDownload = async (downloadId: string) => {
    try {
      await cancelDownloadCommand({ downloadId });
      const next = await getDownloadStatuses();
      setDownloads(next);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const startChat = async () => {
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
  };

  const cancelChat = async () => {
    if (!activeStream) return;
    try {
      await cancelChatStreamCommand({
        streamId: activeStream.streamId,
      });
      const next = await getChatStreamStatuses();
      setChatStatuses(next);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const clearLogs = async () => {
    try {
      await clearLlamaServerLogs();
      setLlamaLogs([]);
      setMessage('llama-server logs cleared.');
    } catch (error) {
      setMessage(String(error));
    }
  };

  const checkProcessHealth = async () => {
    setProcessHealthBusy(true);
    try {
      const health = await tauriBridge.invokeCommand<LlamaServerHealthStatus>(
        'check_llama_server_health',
        { url: `${settings.serverUrl.replace(/\/$/, '')}/health` },
      );
      setProcessHealth(health);
    } catch (error) {
      setMessage(String(error));
    } finally {
      setProcessHealthBusy(false);
    }
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

  return (
    <main className="app-shell">
      <header className="page-header">
        <div>
          <p className="eyebrow">Migration item #n6</p>
          <h1>LlamaCppDesk Tauri control panel</h1>
        </div>
        <button onClick={() => void refreshAll()}>Refresh</button>
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
          <button onClick={() => void saveSettings()}>Save settings</button>
        </article>

        <article className="card">
          <h2>llama-server</h2>
          <p>
            Status: {processStatus.running ? 'Running' : 'Stopped'} (pid:{' '}
            {processStatus.pid ?? '-'})
          </p>
          <div className="health-panel">
            <div className="panel-header">
              <h3>Health</h3>
              <button
                className="secondary"
                onClick={() => void checkProcessHealth()}
                disabled={processHealthBusy}
              >
                {processHealthBusy ? 'Checking...' : 'Check health'}
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
            <button onClick={() => void startProcess()}>Start</button>
            <button onClick={() => void stopProcess()}>Stop</button>
          </div>
          <div className="panel">
            <div className="panel-header">
              <h3>Logs</h3>
              <button onClick={() => void clearLogs()}>Clear logs</button>
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
            <button onClick={() => void savePreset()}>Save preset</button>
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
                      >
                        Edit
                      </button>
                      <button
                        className="danger"
                        onClick={() => void deletePreset(preset.id)}
                      >
                        Delete
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
            <button className="secondary" onClick={() => void clearHistory()}>
              Clear history
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
            <button onClick={() => void appendHistoryEntry()}>
              Append history
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
          <button onClick={() => void startDownload()}>Start download</button>
          <ul>
            {downloads.map((item) => (
              <li key={item.downloadId}>
                <strong>{item.downloadId}</strong> {item.state}{' '}
                {item.percentComplete !== null && item.percentComplete !== undefined
                  ? `(${item.percentComplete.toFixed(1)}%)`
                  : ''}
                <button onClick={() => void cancelDownload(item.downloadId)}>
                  Cancel
                </button>
              </li>
            ))}
          </ul>
        </article>

        <article className="card chat-card">
          <h2>Chat stream</h2>
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
            <button onClick={() => void startChat()}>Start stream</button>
            <button onClick={() => void cancelChat()}>Cancel stream</button>
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
