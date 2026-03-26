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
import { SettingsCard } from './components/SettingsCard';
import { LlamaServerCard } from './components/LlamaServerCard';
import { PresetsCard } from './components/PresetsCard';
import { HistoryCard } from './components/HistoryCard';
import { DownloaderCard } from './components/DownloaderCard';
import { ChatCard } from './components/ChatCard';

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
        <SettingsCard
          settings={settings}
          isSaving={isSavingSettings}
          onServerUrlChange={(value) =>
            setSettings((prev) => ({ ...prev, serverUrl: value }))
          }
          onMaxTokensChange={(value) =>
            setSettings((prev) => ({ ...prev, maxTokens: value }))
          }
          onTemperatureChange={(value) =>
            setSettings((prev) => ({ ...prev, temperature: value }))
          }
          onSave={() => void saveSettings()}
        />

        <LlamaServerCard
          refreshIssue={refreshIssue}
          processStatus={processStatus}
          processHealth={processHealth}
          processPath={processPath}
          processArgs={processArgs}
          isCheckingHealth={isCheckingHealth}
          isProcessTransitionBusy={isProcessTransitionBusy}
          isStartingProcess={isBusy('startProcess')}
          isStoppingProcess={isBusy('stopProcess')}
          isClearingLogs={isClearingLogs}
          onProcessPathChange={setProcessPath}
          onProcessArgsChange={setProcessArgs}
          onCheckHealth={() => void checkProcessHealth()}
          onStartProcess={() => void startProcess()}
          onStopProcess={() => void stopProcess()}
          onClearLogs={() => void clearLogs()}
          llamaLogs={llamaLogs}
        />

        <PresetsCard
          presets={presets}
          presetDraft={presetDraft}
          onNewPreset={resetPresetDraft}
          onResetPresetDraft={resetPresetDraft}
          onPresetDraftChange={setPresetDraft}
          onSavePreset={() => void savePreset()}
          onEditPreset={editPreset}
          onDeletePreset={(presetId) => void deletePreset(presetId)}
          formatTimestamp={formatTimestamp}
          isSaving={isSavingPreset}
          isDeletingPreset={isDeletingPreset}
        />

        <HistoryCard
          historyEntries={historyEntries}
          historyRole={historyRole}
          historyContent={historyContent}
          onRoleChange={setHistoryRole}
          onContentChange={setHistoryContent}
          onClearHistory={() => void clearHistory()}
          onAppendHistoryEntry={() => void appendHistoryEntry()}
          formatTimestamp={formatTimestamp}
          isClearing={isClearingHistory}
          isAppending={isAppendingHistory}
        />

        <DownloaderCard
          downloads={downloads}
          downloadUrl={downloadUrl}
          downloadPath={downloadPath}
          isStartingDownload={isStartingDownload}
          isCancellingDownload={isCancellingDownload}
          onDownloadUrlChange={setDownloadUrl}
          onDownloadPathChange={setDownloadPath}
          onStartDownload={() => void startDownload()}
          onCancelDownload={(downloadId) => void cancelDownload(downloadId)}
        />

        <ChatCard
          chatModel={chatModel}
          chatPrompt={chatPrompt}
          chatStatuses={chatStatuses}
          chatLog={chatLog}
          isStartingChat={isStartingChat}
          isCancellingChat={isCancellingChat}
          canCancelChat={Boolean(activeStream)}
          onChatModelChange={setChatModel}
          onChatPromptChange={setChatPrompt}
          onStartChat={() => void startChat()}
          onCancelChat={() => void cancelChat()}
        />
      </section>

      <footer className="status-line">
        <span>Bridge: {tauriBridge.status}</span>
        <span>{message}</span>
      </footer>
    </main>
  );
}

export default App;
