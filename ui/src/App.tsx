import { tauriBridge } from './lib/tauri';
import { useEffect, useMemo, useState } from 'react';

type Settings = {
  serverUrl: string;
  maxTokens: number;
  temperature: number;
};

type ProcessStatus = {
  running: boolean;
  pid?: number | null;
  lastExitCode?: number | null;
};

type DownloadStatus = {
  downloadId: string;
  sourceUrl: string;
  destinationPath: string;
  state: string;
  bytesDownloaded: number;
  totalBytes?: number | null;
  percentComplete?: number | null;
  error?: string | null;
};

type ChatStreamStatus = {
  streamId: string;
  state: string;
  model: string;
  bytesReceived: number;
  error?: string | null;
};

type ChatEvent = {
  streamId: string;
  eventType: string;
  data?: string;
  state: string;
  error?: string;
};

function App() {
  const [settings, setSettings] = useState<Settings>({
    serverUrl: 'http://127.0.0.1:8080',
    maxTokens: 512,
    temperature: 0.7,
  });
  const [processPath, setProcessPath] = useState('llama-server');
  const [processArgs, setProcessArgs] = useState('--port 8080');
  const [processStatus, setProcessStatus] = useState<ProcessStatus>({
    running: false,
  });

  const [downloadUrl, setDownloadUrl] = useState('');
  const [downloadPath, setDownloadPath] = useState('');
  const [downloads, setDownloads] = useState<DownloadStatus[]>([]);

  const [chatModel, setChatModel] = useState('default');
  const [chatPrompt, setChatPrompt] = useState('');
  const [chatLog, setChatLog] = useState<string[]>([]);
  const [chatStatuses, setChatStatuses] = useState<ChatStreamStatus[]>([]);

  const [message, setMessage] = useState('');

  const activeStream = useMemo(
    () => chatStatuses.find((item) => item.state === 'streaming'),
    [chatStatuses],
  );

  useEffect(() => {
    const stopListening = tauriBridge.listenToEvent(
      'chat_stream_event',
      (payload) => {
        const event = payload as ChatEvent;
        if (event.eventType === 'chunk' && event.data) {
          setChatLog((prev) => [...prev, event.data as string]);
        }
        if (event.eventType === 'error' && event.error) {
          setMessage(event.error);
        }
      },
    );

    void refreshAll();
    return () => stopListening();
  }, []);

  const refreshAll = async () => {
    try {
      const [loadedSettings, loadedProcess, loadedDownloads, loadedStreams] =
        await Promise.all([
          tauriBridge.invokeCommand<Settings>('get_settings'),
          tauriBridge.invokeCommand<ProcessStatus>('get_llama_server_status'),
          tauriBridge.invokeCommand<DownloadStatus[]>('get_download_statuses'),
          tauriBridge.invokeCommand<ChatStreamStatus[]>(
            'get_chat_stream_statuses',
          ),
        ]);
      setSettings(loadedSettings);
      setProcessStatus(loadedProcess);
      setDownloads(loadedDownloads);
      setChatStatuses(loadedStreams);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const saveSettings = async () => {
    try {
      await tauriBridge.invokeCommand('save_settings', { settings });
      setMessage('Settings saved.');
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
      const status = await tauriBridge.invokeCommand<ProcessStatus>(
        'start_llama_server',
        { executablePath: processPath, args },
      );
      setProcessStatus(status);
      setMessage('llama-server started.');
    } catch (error) {
      setMessage(String(error));
    }
  };

  const stopProcess = async () => {
    try {
      const status =
        await tauriBridge.invokeCommand<ProcessStatus>('stop_llama_server');
      setProcessStatus(status);
      setMessage('llama-server stopped.');
    } catch (error) {
      setMessage(String(error));
    }
  };

  const startDownload = async () => {
    try {
      await tauriBridge.invokeCommand<DownloadStatus>('start_download', {
        sourceUrl: downloadUrl,
        destinationPath: downloadPath,
      });
      const next =
        await tauriBridge.invokeCommand<DownloadStatus[]>('get_download_statuses');
      setDownloads(next);
      setMessage('Download started.');
    } catch (error) {
      setMessage(String(error));
    }
  };

  const cancelDownload = async (downloadId: string) => {
    try {
      await tauriBridge.invokeCommand('cancel_download', { downloadId });
      const next =
        await tauriBridge.invokeCommand<DownloadStatus[]>('get_download_statuses');
      setDownloads(next);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const startChat = async () => {
    try {
      setChatLog([]);
      await tauriBridge.invokeCommand<ChatStreamStatus>('start_chat_stream', {
        request: {
          serverUrl: settings.serverUrl,
          model: chatModel,
          messages: [{ role: 'user', content: chatPrompt }],
          maxTokens: settings.maxTokens,
          temperature: settings.temperature,
        },
      });
      const next =
        await tauriBridge.invokeCommand<ChatStreamStatus[]>(
          'get_chat_stream_statuses',
        );
      setChatStatuses(next);
      setMessage('Chat stream started.');
    } catch (error) {
      setMessage(String(error));
    }
  };

  const cancelChat = async () => {
    if (!activeStream) return;
    try {
      await tauriBridge.invokeCommand('cancel_chat_stream', {
        streamId: activeStream.streamId,
      });
      const next =
        await tauriBridge.invokeCommand<ChatStreamStatus[]>(
          'get_chat_stream_statuses',
        );
      setChatStatuses(next);
    } catch (error) {
      setMessage(String(error));
    }
  };

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
                {item.percentComplete ? `(${item.percentComplete.toFixed(1)}%)` : ''}
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
