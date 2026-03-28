import type { DownloadStatus } from '../lib/tauri/api';
import { type FailureView, getFailureView } from '../utils/failure';

export type DownloaderCardProps = {
  downloads: DownloadStatus[];
  downloadUrl: string;
  isStartingDownload: boolean;
  isCancellingDownload: (downloadId: string) => boolean;
  onDownloadUrlChange: (value: string) => void;
  onStartDownload: () => void;
  onCancelDownload: (downloadId: string) => void;
  downloadSource: string;
  hfToken: string;
  hfFiles: string[];
  ollamaTags: string[];
  selectedHfFile: string;
  selectedOllamaTag: string;
  downloadFileName: string;
  onDownloadSourceChange: (source: string) => void;
  onHfTokenChange: (token: string) => void;
  onLoadHfFiles: () => void;
  onLoadOllamaTags: () => void;
  onSelectedHfFileChange: (file: string) => void;
  onSelectedOllamaTagChange: (tag: string) => void;
  onDownloadFileNameChange: (name: string) => void;
};

const DOWNLOAD_SOURCES = [
  { value: 'direct', label: 'Direct URL' },
  { value: 'huggingface', label: 'Hugging Face' },
  { value: 'ollama', label: 'Ollama Library' },
];

const getInputLabel = (source: string) => {
  switch (source) {
    case 'huggingface':
      return 'Repository (e.g. TheBloke/Llama-2-7B-GGUF)';
    case 'ollama':
      return 'Model name (e.g. llama2)';
    default:
      return 'Source URL';
  }
};

const getInputPlaceholder = (source: string) => {
  switch (source) {
    case 'huggingface':
      return 'owner/repo';
    case 'ollama':
      return 'model-name';
    default:
      return 'https://...';
  }
};

const getToneForState = (value: string) => {
  switch (value.toLowerCase()) {
    case 'downloading':
      return 'info';
    case 'completed':
      return 'success';
    case 'cancelled':
      return 'neutral';
    case 'failed':
      return 'danger';
    default:
      return 'neutral';
  }
};

const formatStatusLabel = (value: string) =>
  value
    .replace(/_/g, ' ')
    .replace(/\b\w/g, (character) => character.toUpperCase());

export function DownloaderCard({
  downloads,
  downloadUrl,
  isStartingDownload,
  isCancellingDownload,
  onDownloadUrlChange,
  onStartDownload,
  onCancelDownload,
  downloadSource,
  hfToken,
  hfFiles,
  ollamaTags,
  selectedHfFile,
  selectedOllamaTag,
  downloadFileName,
  onDownloadSourceChange,
  onHfTokenChange,
  onLoadHfFiles,
  onLoadOllamaTags,
  onSelectedHfFileChange,
  onSelectedOllamaTagChange,
  onDownloadFileNameChange,
}: DownloaderCardProps) {
  return (
    <article className="card">
      <h2>Downloader</h2>
      <div className="status-summary">
        <span className="status-badge neutral">
          {downloads.length ? `${downloads.length} tracked` : 'No downloads'}
        </span>
        <span className="hint">State updates refresh automatically</span>
      </div>
      <label>
        Download source
        <select
          value={downloadSource}
          onChange={(e) => onDownloadSourceChange(e.target.value)}
        >
          {DOWNLOAD_SOURCES.map((src) => (
            <option key={src.value} value={src.value}>
              {src.label}
            </option>
          ))}
        </select>
      </label>
      <label>
        {getInputLabel(downloadSource)}
        <input
          value={downloadUrl}
          onChange={(e) => onDownloadUrlChange(e.target.value)}
          placeholder={getInputPlaceholder(downloadSource)}
        />
      </label>
      {downloadSource === 'huggingface' && (
        <>
          <label>
            HF token (optional)
            <input
              type="password"
              value={hfToken}
              onChange={(e) => onHfTokenChange(e.target.value)}
              placeholder="hf_..."
              autoComplete="off"
            />
          </label>
          <div className="row">
            <button type="button" onClick={onLoadHfFiles}>
              Load files
            </button>
          </div>
          {hfFiles.length > 0 && (
            <label>
              Select file
              <select
                value={selectedHfFile}
                onChange={(e) => onSelectedHfFileChange(e.target.value)}
              >
                <option value="">-- choose a file --</option>
                {hfFiles.map((file) => (
                  <option key={file} value={file}>
                    {file}
                  </option>
                ))}
              </select>
            </label>
          )}
        </>
      )}
      {downloadSource === 'ollama' && (
        <>
          <div className="row">
            <button type="button" onClick={onLoadOllamaTags}>
              Load tags
            </button>
          </div>
          {ollamaTags.length > 0 && (
            <label>
              Select tag
              <select
                value={selectedOllamaTag}
                onChange={(e) => onSelectedOllamaTagChange(e.target.value)}
              >
                <option value="">-- choose a tag --</option>
                {ollamaTags.map((tag) => (
                  <option key={tag} value={tag}>
                    {tag}
                  </option>
                ))}
              </select>
            </label>
          )}
        </>
      )}
      <label>
        Filename override (optional)
        <input
          value={downloadFileName}
          onChange={(e) => onDownloadFileNameChange(e.target.value)}
          placeholder="Leave blank for suggested filename"
        />
      </label>
      <button onClick={() => void onStartDownload()} disabled={isStartingDownload}>
        {isStartingDownload ? 'Starting...' : 'Start download'}
      </button>
      <ul>
        {downloads.map((item) => {
          const failureView = item.error
            ? getFailureView(item.error)
            : item.state === 'failed'
              ? {
                  label: 'Failure',
                  tone: 'danger' as const,
                  detailClass: 'failure' as const,
                }
              : null;

          return (
            <li key={item.downloadId}>
              <div className="status-summary">
                <span className={`status-badge ${getToneForState(item.state)}`}>
                  {formatStatusLabel(item.state)}
                </span>
                <strong>{item.downloadId}</strong>
              </div>
              {failureView ? (
                <div className={`status-detail ${failureView.detailClass}`}>
                  <span className={`status-badge ${failureView.tone}`}>
                    {failureView.label}
                  </span>
                  <p className="status-detail-text">
                    {item.error ?? 'The download ended in a failed state.'}
                  </p>
                </div>
              ) : null}
              {item.sourceUrl ? <div className="hint">{item.sourceUrl}</div> : null}
              {item.destinationPath ? (
                <div className="hint">{item.destinationPath}</div>
              ) : null}
              <div className="hint">
                {item.percentComplete !== null && item.percentComplete !== undefined
                  ? `${item.percentComplete.toFixed(1)}%`
                  : 'Progress unavailable'}
              </div>
              <div className="row">
                <button
                  onClick={() => void onCancelDownload(item.downloadId)}
                  disabled={isCancellingDownload(item.downloadId)}
                >
                  {isCancellingDownload(item.downloadId) ? 'Cancelling...' : 'Cancel'}
                </button>
              </div>
            </li>
          );
        })}
      </ul>
    </article>
  );
}
