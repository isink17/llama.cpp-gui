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
  waitForServerReady,
  pickFile,
  pickFolder,
  resolveModelReference,
  listHuggingFaceFiles,
  listOllamaTags,
  type ChatStreamEvent,
  type ChatStreamStatus,
  type DownloadStatus,
  type HistoryEntry,
  type LlamaProcessStatus,
  type LlamaServerHealthStatus,
  type Preset,
  type Settings,
} from './lib/tauri/api';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { SettingsCard } from './components/SettingsCard';
import { LlamaServerCard } from './components/LlamaServerCard';
import { PresetsCard } from './components/PresetsCard';
import { HistoryCard } from './components/HistoryCard';
import { DownloaderCard } from './components/DownloaderCard';
import { ChatCard } from './components/ChatCard';

const LOG_LIMIT = 200;
/** Interval for frequently changing data: process status, downloads, chat, logs. */
const FAST_REFRESH_MS = 4000;
/** Interval for mostly-static data: settings, presets, history. */
const SLOW_REFRESH_MS = 60000;
const INITIAL_REFRESH_RETRIES = 2;
const INITIAL_REFRESH_RETRY_DELAY_MS = 1500;
const HISTORY_ROLES: HistoryEntry['role'][] = ['system', 'user', 'assistant'];
const MAX_PROMPT_HISTORY = 40;

const createId = (prefix: string) =>
  `${prefix}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;

const formatTimestamp = (value: string) => {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString();
};

function App() {
  const [settings, setSettings] = useState<Settings>({
    llamaServerPath: '',
    modelPath: '',
    host: '127.0.0.1',
    port: 8080,
    contextSize: 4096,
    threads: 4,
    gpuLayers: 0,
    temperature: 0.7,
    maxTokens: 512,
    downloadFolder: '',
    recentModelPaths: [],
    recentServerPaths: [],
    recentModelUrls: [],
  });
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
  const [downloads, setDownloads] = useState<DownloadStatus[]>([]);
  const [llamaLogs, setLlamaLogs] = useState<string[]>([]);

  const [presets, setPresets] = useState<Preset[]>([]);
  const emptyPresetDraft: Preset = {
    id: '',
    name: '',
    modelPath: '',
    host: '127.0.0.1',
    port: 8080,
    contextSize: 4096,
    threads: 4,
    gpuLayers: 0,
    temperature: 0.7,
    maxTokens: 512,
  };
  const [presetDraft, setPresetDraft] = useState<Preset>(emptyPresetDraft);

  const [historyEntries, setHistoryEntries] = useState<HistoryEntry[]>([]);
  const [historyRole, setHistoryRole] = useState<HistoryEntry['role']>('user');
  const [historyContent, setHistoryContent] = useState('');

  const [chatModel, setChatModel] = useState('default');
  const [chatPrompt, setChatPrompt] = useState('');
  const [chatLog, setChatLog] = useState<string[]>([]);
  const [chatStatuses, setChatStatuses] = useState<ChatStreamStatus[]>([]);

  const [downloadSource, setDownloadSource] = useState('direct');
  const [hfToken, setHfToken] = useState('');
  const [hfFiles, setHfFiles] = useState<string[]>([]);
  const [ollamaTags, setOllamaTags] = useState<string[]>([]);
  const [selectedHfFile, setSelectedHfFile] = useState('');
  const [selectedOllamaTag, setSelectedOllamaTag] = useState('');
  const [downloadFileName, setDownloadFileName] = useState('');

  const [promptHistory, setPromptHistory] = useState<string[]>([]);
  const [promptHistoryIndex, setPromptHistoryIndex] = useState(-1);
  const [promptDraftBeforeHistory, setPromptDraftBeforeHistory] = useState('');

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

  const safeErrorMessage = useCallback(
    (error: unknown): string => {
      let msg = String(error);
      if (hfToken) {
        msg = msg.replaceAll(hfToken, '***');
      }
      // Strip file system paths from error messages
      msg = msg.replace(/[A-Z]:\\\\[^\s"']*/gi, '[path]');
      msg = msg.replace(/\/(?:home|Users|tmp|var|etc|usr|opt)\/[^\s"']*/g, '[path]');
      return msg;
    },
    [hfToken],
  );

  const refreshDynamic = useCallback(async () => {
    await withBusyAction('refreshDynamic', async () => {
      try {
        const [loadedProcess, loadedDownloads, loadedStreams, loadedLogs] =
          await Promise.all([
            getLlamaServerStatus(),
            getDownloadStatuses(),
            getChatStreamStatuses(),
            getLlamaServerLogs({ limit: LOG_LIMIT }),
          ]);
        setProcessStatus(loadedProcess);
        setDownloads(loadedDownloads);
        setChatStatuses(loadedStreams);
        setLlamaLogs(loadedLogs);
      } catch {
        // dynamic refresh failures are transient; don't update UI state
      }
    });
  }, [withBusyAction]);

  const refreshStatic = useCallback(async () => {
    await withBusyAction('refreshStatic', async () => {
      try {
        const [loadedSettings, loadedPresets, loadedHistory] =
          await Promise.all([getSettings(), getPresets(), getHistory()]);
        setSettings(loadedSettings);
        setPresets(loadedPresets);
        setHistoryEntries(loadedHistory);
      } catch {
        // static refresh failures are transient; don't update UI state
      }
    });
  }, [withBusyAction]);

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
        const errorMessage = safeErrorMessage(error);
        setRefreshIssue(errorMessage);
        setMessage(errorMessage);
      }
    });
  }, [withBusyAction, safeErrorMessage]);

  useEffect(() => {
    let cancelled = false;

    const listenPromise = subscribeToChatStreamEvent(
      (event: ChatStreamEvent) => {
        if (event.eventType === 'chunk' && event.data) {
          setChatLog((prev) => [...prev, event.data!]);
        }
        if (event.eventType === 'error' && event.error) {
          setMessage(safeErrorMessage(event.error));
        }
      },
    );

    const initialLoad = async () => {
      for (let attempt = 0; attempt <= INITIAL_REFRESH_RETRIES; attempt++) {
        if (cancelled) return;
        try {
          await refreshAll();
          return;
        } catch {
          if (attempt < INITIAL_REFRESH_RETRIES) {
            await new Promise((r) =>
              setTimeout(r, INITIAL_REFRESH_RETRY_DELAY_MS),
            );
          }
        }
      }
      if (!cancelled) {
        setRefreshIssue('Initial load failed after retries');
        setMessage('Failed to load data from backend.');
      }
    };

    void initialLoad();

    let fastIntervalId = window.setInterval(() => {
      void refreshDynamic();
    }, FAST_REFRESH_MS);

    let slowIntervalId = window.setInterval(() => {
      void refreshStatic();
    }, SLOW_REFRESH_MS);

    const handleVisibilityChange = () => {
      if (document.hidden) {
        if (fastIntervalId) clearInterval(fastIntervalId);
        if (slowIntervalId) clearInterval(slowIntervalId);
      } else {
        // Immediately refresh, then restart intervals
        void refreshDynamic();
        void refreshStatic();
        fastIntervalId = window.setInterval(() => void refreshDynamic(), FAST_REFRESH_MS);
        slowIntervalId = window.setInterval(() => void refreshStatic(), SLOW_REFRESH_MS);
      }
    };
    document.addEventListener('visibilitychange', handleVisibilityChange);

    return () => {
      cancelled = true;
      window.clearInterval(fastIntervalId);
      window.clearInterval(slowIntervalId);
      document.removeEventListener('visibilitychange', handleVisibilityChange);
      void listenPromise.then((dispose) => dispose());
    };
  }, [refreshAll, refreshDynamic, refreshStatic, safeErrorMessage]);

  const saveSettings = async () => {
    await withBusyAction('saveSettings', async () => {
      try {
        await saveSettingsCommand(settings);
        setMessage('Settings saved.');
      } catch (error) {
        setMessage(safeErrorMessage(error));
      }
    });
  };

  const resetPresetDraft = () => {
    setPresetDraft(emptyPresetDraft);
  };

  const editPreset = (preset: Preset) => {
    setPresetDraft(preset);
  };

  const savePresetFromSettings = async () => {
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
          modelPath: settings.modelPath,
          host: settings.host,
          port: settings.port,
          contextSize: settings.contextSize,
          threads: settings.threads,
          gpuLayers: settings.gpuLayers,
          temperature: settings.temperature,
          maxTokens: settings.maxTokens,
        };

        const next = await savePresetCommand({
          preset: payload,
        });
        setPresets(next);
        setPresetDraft(payload);
        setMessage(`Preset '${payload.name}' saved.`);
      } catch (error) {
        setMessage(safeErrorMessage(error));
      }
    });
  };

  const applyPreset = (preset: Preset) => {
    setSettings((prev) => ({
      ...prev,
      modelPath: preset.modelPath,
      host: preset.host,
      port: preset.port,
      contextSize: preset.contextSize,
      threads: preset.threads,
      gpuLayers: preset.gpuLayers,
      temperature: preset.temperature,
      maxTokens: preset.maxTokens,
    }));
    setMessage(`Preset '${preset.name}' applied.`);
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
        setMessage(safeErrorMessage(error));
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
        setMessage(safeErrorMessage(error));
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
        setMessage(safeErrorMessage(error));
      }
    });
  };

  const buildServerArgs = (): string[] => {
    const args = [
      '--host', settings.host,
      '--port', String(settings.port),
      '-m', settings.modelPath,
      '-c', String(settings.contextSize),
      '-t', String(settings.threads),
    ];
    if (settings.gpuLayers > 0) {
      args.push('--n-gpu-layers', String(settings.gpuLayers));
    }
    return args;
  };

  const serverUrl = `http://${settings.host}:${settings.port}`;

  const startProcess = async () => {
    await withBusyAction('startProcess', async () => {
      try {
        const status = await startLlamaServerCommand({
          executablePath: settings.llamaServerPath,
          args: buildServerArgs(),
        });
        setProcessStatus(status);
        setProcessHealth(null);
        setMessage('llama-server started. Waiting for readiness...');

        try {
          const health = await waitForServerReady({ serverUrl });
          setProcessHealth(health);
          setMessage('Server ready.');
        } catch (readyError) {
          setMessage(`Server started but not ready: ${safeErrorMessage(readyError)}`);
        }
      } catch (error) {
        setMessage(safeErrorMessage(error));
      }
    });
  };

  const restartProcess = async () => {
    await withBusyAction('restartProcess', async () => {
      try {
        const stoppedStatus = await stopLlamaServerCommand();
        setProcessStatus(stoppedStatus);
        setProcessHealth(null);

        const startedStatus = await startLlamaServerCommand({
          executablePath: settings.llamaServerPath,
          args: buildServerArgs(),
        });
        setProcessStatus(startedStatus);
        setProcessHealth(null);
        setMessage('llama-server restarted. Waiting for readiness...');

        try {
          const health = await waitForServerReady({ serverUrl });
          setProcessHealth(health);
          setMessage('Server ready.');
        } catch (readyError) {
          setMessage(`Server restarted but not ready: ${safeErrorMessage(readyError)}`);
        }
      } catch (error) {
        setMessage(safeErrorMessage(error));
      }
    });
  };

  const stopApplication = async () => {
    await withBusyAction('stopApplication', async () => {
      try {
        if (processStatus.running) {
          try {
            await stopLlamaServerCommand();
          } catch {
            // Closing the app should still be allowed even if shutdown fails.
          }
        }

        await getCurrentWindow().close();
      } catch (error) {
        setMessage(safeErrorMessage(error));
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
        setMessage(safeErrorMessage(error));
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
        setMessage(safeErrorMessage(error));
      }
    });
  };

  const startChat = async () => {
    await withBusyAction('startChat', async () => {
      try {
        setChatLog([]);
        await startChatStreamCommand({
          request: {
            serverUrl: serverUrl,
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
        setMessage(safeErrorMessage(error));
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
        setMessage(safeErrorMessage(error));
      }
    });
  };

  const trackPromptHistory = useCallback((prompt: string) => {
    setPromptHistory((prev) => {
      const filtered = prev.filter((entry) => entry !== prompt);
      const next = [...filtered, prompt];
      if (next.length > MAX_PROMPT_HISTORY) {
        return next.slice(next.length - MAX_PROMPT_HISTORY);
      }
      return next;
    });
  }, []);

  const navigatePromptHistory = useCallback(
    (direction: number) => {
      setPromptHistoryIndex((prevIndex) => {
        const length = promptHistory.length;
        if (length === 0) return -1;

        if (prevIndex === -1 && direction === -1) {
          setPromptDraftBeforeHistory(chatPrompt);
          const newIndex = length - 1;
          setChatPrompt(promptHistory[newIndex]);
          return newIndex;
        }

        const newIndex = prevIndex + direction;

        if (newIndex < 0) {
          setChatPrompt(promptHistory[0]);
          return 0;
        }

        if (newIndex >= length) {
          setChatPrompt(promptDraftBeforeHistory);
          setPromptDraftBeforeHistory('');
          return -1;
        }

        setChatPrompt(promptHistory[newIndex]);
        return newIndex;
      });
    },
    [promptHistory, chatPrompt, promptDraftBeforeHistory],
  );

  const resetPromptHistoryNavigation = useCallback(() => {
    setPromptHistoryIndex(-1);
    setPromptDraftBeforeHistory('');
  }, []);

  const handleSlashCommand = useCallback(
    (text: string): boolean => {
      const trimmed = text.trim().toLowerCase();
      if (!trimmed.startsWith('/')) return false;

      switch (trimmed) {
        case '/help':
          setMessage('Commands: /help, /clear, /copylast, /stop, /status, /reuselast');
          return true;
        case '/clear':
          setChatLog([]);
          setMessage('Chat cleared.');
          return true;
        case '/copylast': {
          const lastAssistantText = [...chatLog].reverse().find((chunk) => chunk.length > 0);
          if (lastAssistantText) {
            void navigator.clipboard.writeText(chatLog.join(''));
            setMessage('Last reply copied.');
          } else {
            setMessage('No reply to copy.');
          }
          return true;
        }
        case '/stop':
          if (activeStream) {
            void cancelChat();
            setMessage('Stopping generation...');
          } else {
            setMessage('No active stream to stop.');
          }
          return true;
        case '/status':
          setMessage(
            processStatus.running
              ? `Server is running (pid: ${processStatus.pid ?? 'unknown'}).`
              : 'Server is not running.',
          );
          return true;
        case '/reuselast': {
          const replyText = chatLog.join('');
          if (replyText.length > 0) {
            setChatPrompt(replyText);
            setMessage('Last reply moved to prompt.');
          } else {
            setMessage('No reply to reuse.');
          }
          return true;
        }
        default:
          return false;
      }
    },
    [chatLog, activeStream, cancelChat, processStatus],
  );

  const sendChatPrompt = useCallback(async () => {
    const trimmed = chatPrompt.trim();
    if (!trimmed) return;

    if (handleSlashCommand(trimmed)) {
      setChatPrompt('');
      return;
    }

    if (activeStream) {
      setMessage('A chat stream is already active.');
      return;
    }

    trackPromptHistory(trimmed);
    resetPromptHistoryNavigation();
    await startChat();
  }, [
    chatPrompt,
    handleSlashCommand,
    activeStream,
    trackPromptHistory,
    resetPromptHistoryNavigation,
    startChat,
  ]);

  const clearLogs = async () => {
    await withBusyAction('clearLogs', async () => {
      try {
        await clearLlamaServerLogs();
        setLlamaLogs([]);
        setMessage('llama-server logs cleared.');
      } catch (error) {
        setMessage(safeErrorMessage(error));
      }
    });
  };

  const checkProcessHealth = async () => {
    await withBusyAction('checkHealth', async () => {
      try {
        const health = await getLlamaServerHealth({
          url: `${serverUrl.replace(/\/$/, '')}/health`,
        });
        setProcessHealth(health);
      } catch (error) {
        const healthUrl = `${serverUrl.replace(/\/$/, '')}/health`;
        const errorMessage = safeErrorMessage(error);
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

  const browseServerPath = async () => {
    try {
      const result = await pickFile({
        title: 'Select llama-server executable',
        extensions: ['exe', ''],
      });
      if (result) {
        setSettings((prev) => ({ ...prev, llamaServerPath: result }));
      }
    } catch (error) {
      setMessage(safeErrorMessage(error));
    }
  };

  const browseModelPath = async () => {
    try {
      const result = await pickFile({
        title: 'Select GGUF model file',
        extensions: ['gguf'],
      });
      if (result) {
        setSettings((prev) => ({ ...prev, modelPath: result }));
      }
    } catch (error) {
      setMessage(safeErrorMessage(error));
    }
  };

  const browseDownloadFolder = async () => {
    try {
      const result = await pickFolder({
        title: 'Select download folder',
      });
      if (result) {
        setSettings((prev) => ({ ...prev, downloadFolder: result }));
      }
    } catch (error) {
      setMessage(safeErrorMessage(error));
    }
  };

  const loadHfFiles = async () => {
    await withBusyAction('loadHfFiles', async () => {
      try {
        const files = await listHuggingFaceFiles({
          input: downloadUrl,
          hfToken: hfToken || null,
        });
        setHfFiles(files);
        if (files.length > 0) {
          setSelectedHfFile(files[0]);
        }
        setMessage(`Loaded ${files.length} file(s) from repository.`);
      } catch (error) {
        setMessage(safeErrorMessage(error));
      }
    });
  };

  const loadOllamaTags = async () => {
    await withBusyAction('loadOllamaTags', async () => {
      try {
        const tags = await listOllamaTags({ input: downloadUrl });
        setOllamaTags(tags);
        if (tags.length > 0) {
          setSelectedOllamaTag(tags[0]);
        }
        setMessage(`Loaded ${tags.length} tag(s).`);
      } catch (error) {
        setMessage(safeErrorMessage(error));
      }
    });
  };

  const startDownloadWithResolve = async () => {
    await withBusyAction('startDownload', async () => {
      try {
        let resolvedUrl = downloadUrl;
        let fileName = downloadFileName;
        let headers: Record<string, string> | null | undefined;

        if (downloadSource === 'huggingface' || downloadSource === 'ollama') {
          const sourceLabel =
            downloadSource === 'huggingface' ? 'Hugging Face' : 'Ollama Library';
          const input =
            downloadSource === 'huggingface'
              ? selectedHfFile
                ? `${downloadUrl}::${selectedHfFile}`
                : downloadUrl
              : selectedOllamaTag
                ? `${downloadUrl}:${selectedOllamaTag}`
                : downloadUrl;

          const resolved = await resolveModelReference({
            source: sourceLabel,
            input,
            hfToken: hfToken || null,
          });

          resolvedUrl = resolved.downloadUrl;
          headers = resolved.requestHeaders;
          if (!fileName) {
            fileName = resolved.suggestedFileName;
          }
        }

        const destFolder = settings.downloadFolder;
        if (!destFolder) {
          setMessage('Download folder must be set in settings.');
          return;
        }
        if (!fileName) {
          setMessage('Could not determine filename for download.');
          return;
        }
        const destPath = `${destFolder.replace(/[\\/]$/, '')}/${fileName}`;

        await startDownloadCommand({
          sourceUrl: resolvedUrl,
          destinationPath: destPath,
          requestHeaders: headers ?? null,
        });
        const next = await getDownloadStatuses();
        setDownloads(next);
        setMessage('Download started.');
      } catch (error) {
        setMessage(safeErrorMessage(error));
      }
    });
  };

  const isRefreshing = isBusy('refresh');
  const isSavingSettings = isBusy('saveSettings');
  const isCheckingHealth = isBusy('checkHealth');
  const isProcessTransitionBusy =
    isBusy('startProcess') || isBusy('stopProcess') || isBusy('restartProcess');
  const isStoppingApplication = isBusy('stopApplication');
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
                  : refreshIssue
                    ? `Error: ${refreshIssue}`
                    : 'Waiting...'}
            </span>
          </div>
        </div>
        <div className="row">
          <button type="button" onClick={() => void refreshAll()} disabled={isRefreshing}>
            {isRefreshing ? 'Refreshing...' : 'Refresh'}
          </button>
          <button
            type="button"
            className="danger"
            onClick={() => void stopApplication()}
            disabled={isStoppingApplication}
          >
            {isStoppingApplication ? 'Stopping...' : 'Stop application'}
          </button>
        </div>
      </header>

      <section className="grid">
        <SettingsCard
          settings={settings}
          isSaving={isSavingSettings}
          onChange={(patch) =>
            setSettings((prev) => ({ ...prev, ...patch }))
          }
          onSave={() => void saveSettings()}
          onBrowseServerPath={() => void browseServerPath()}
          onBrowseModelPath={() => void browseModelPath()}
          onBrowseDownloadFolder={() => void browseDownloadFolder()}
        />

        <LlamaServerCard
          refreshIssue={refreshIssue}
          processStatus={processStatus}
          processHealth={processHealth}
          serverUrl={serverUrl}
          isCheckingHealth={isCheckingHealth}
          isProcessTransitionBusy={isProcessTransitionBusy}
          isStartingProcess={isBusy('startProcess')}
          isStoppingProcess={isBusy('stopProcess')}
          isRestartingProcess={isBusy('restartProcess')}
          isClearingLogs={isClearingLogs}
          onCheckHealth={() => void checkProcessHealth()}
          onStartProcess={() => void startProcess()}
          onRestartProcess={() => void restartProcess()}
          onStopProcess={() => void stopProcess()}
          onClearLogs={() => void clearLogs()}
          llamaLogs={llamaLogs}
        />

        <PresetsCard
          presets={presets}
          presetDraft={presetDraft}
          settings={settings}
          onNewPreset={resetPresetDraft}
          onResetPresetDraft={resetPresetDraft}
          onPresetDraftChange={setPresetDraft}
          onSavePresetFromSettings={() => void savePresetFromSettings()}
          onApplyPreset={applyPreset}
          onEditPreset={editPreset}
          onDeletePreset={(presetId) => void deletePreset(presetId)}
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
          isStartingDownload={isStartingDownload}
          isCancellingDownload={isCancellingDownload}
          onDownloadUrlChange={setDownloadUrl}
          onStartDownload={() => void startDownloadWithResolve()}
          onCancelDownload={(downloadId) => void cancelDownload(downloadId)}
          downloadSource={downloadSource}
          hfToken={hfToken}
          hfFiles={hfFiles}
          ollamaTags={ollamaTags}
          selectedHfFile={selectedHfFile}
          selectedOllamaTag={selectedOllamaTag}
          downloadFileName={downloadFileName}
          onDownloadSourceChange={setDownloadSource}
          onHfTokenChange={setHfToken}
          onLoadHfFiles={() => void loadHfFiles()}
          onLoadOllamaTags={() => void loadOllamaTags()}
          onSelectedHfFileChange={setSelectedHfFile}
          onSelectedOllamaTagChange={setSelectedOllamaTag}
          onDownloadFileNameChange={setDownloadFileName}
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
          onStartChat={() => void sendChatPrompt()}
          onCancelChat={() => void cancelChat()}
          onUseAsPrompt={(text) => setChatPrompt(text)}
          onSendPrompt={() => void sendChatPrompt()}
          onNavigateHistory={navigatePromptHistory}
          onCancelGeneration={() => void cancelChat()}
        />
      </section>

      <footer className="status-line" aria-live="polite">
        <span>Bridge: {tauriBridge.status}</span>
        <span>{message}</span>
      </footer>
    </main>
  );
}

export default App;
