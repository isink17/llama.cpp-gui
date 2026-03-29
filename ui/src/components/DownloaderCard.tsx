import type { DownloadStatus } from '../lib/tauri/api';
import { type FailureView, getFailureView } from '../utils/failure';

export type DownloaderCardProps = {
  downloads: DownloadStatus[];
  downloadUrl: string;
  canUseBackend: boolean;
  isDesktop: boolean;
  downloadFolder: string;
  downloadFolderSourceLabel: string;
  isStartingDownload: boolean;
  isCancellingDownload: (downloadId: string) => boolean;
  onDownloadUrlChange: (value: string) => void;
  onDownloadFolderChange: (value: string) => void;
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

const formatSourceSummary = (
  source: string,
  downloadUrl: string,
  selectedHfFile: string,
  selectedOllamaTag: string,
) => {
  const trimmedUrl = downloadUrl.trim();
  switch (source) {
    case 'huggingface':
      return trimmedUrl
        ? `${trimmedUrl}${selectedHfFile ? ` / ${selectedHfFile}` : ''}`
        : 'Enter a Hugging Face repository';
    case 'ollama':
      return trimmedUrl
        ? `${trimmedUrl}${selectedOllamaTag ? `:${selectedOllamaTag}` : ''}`
        : 'Enter an Ollama model name';
    default:
      return trimmedUrl || 'Enter a source URL';
  }
};

const buildPreviewPath = (downloadFolder: string, downloadFileName: string) => {
  const folder = downloadFolder.trim().replace(/[\\/]+$/, '');
  const file = downloadFileName.trim();
  if (!folder && !file) {
    return 'Set a destination folder and optional filename';
  }
  if (!folder) {
    return 'Set a destination folder first';
  }
  return `${folder}/${file || 'suggested-filename'}`;
};

export function DownloaderCard({
  downloads,
  downloadUrl,
  canUseBackend,
  isDesktop,
  downloadFolder,
  downloadFolderSourceLabel,
  isStartingDownload,
  isCancellingDownload,
  onDownloadUrlChange,
  onDownloadFolderChange,
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
  const isReadOnly = !canUseBackend;
  const browserMode = !isDesktop;
  const sourceSummary = formatSourceSummary(
    downloadSource,
    downloadUrl,
    selectedHfFile,
    selectedOllamaTag,
  );
  const previewPath = buildPreviewPath(downloadFolder, downloadFileName);
  const folderMissing = browserMode && !downloadFolder.trim();
  const canStartDownload = !isStartingDownload && !isReadOnly && (!browserMode || Boolean(downloadFolder.trim()));
  const startButtonLabel = isStartingDownload
    ? 'Starting...'
    : folderMissing
      ? 'Set folder first'
      : 'Start download';

  return (
    <article className="card">
      <div className="panel-header">
        <div>
          <p className="card-kicker">Remote file transfers</p>
          <h2>Downloader</h2>
        </div>
        <span className={`status-badge ${browserMode ? 'info' : 'neutral'}`}>
          {browserMode ? 'Browser-safe' : 'Desktop'}
        </span>
      </div>
      {browserMode ? (
        <div className="status-detail timeout">
          <span className="status-badge info">Destination</span>
          <p className="status-detail-text">
            Browser mode cannot open a folder picker. Set the download folder here
            so files land in a predictable place.
          </p>
          <label>
            Download folder
            <input
              value={downloadFolder}
              onChange={(e) => onDownloadFolderChange(e.target.value)}
              placeholder="Folder for downloaded models"
              disabled={isReadOnly}
            />
          </label>
          <div className="field-source">{downloadFolderSourceLabel}</div>
        </div>
      ) : null}
      <div className="status-detail">
        <span className="status-badge info">Download recipe</span>
        <div className="recipe-grid">
          <div className="recipe-item">
            <span className="hint">Source</span>
            <strong>{sourceSummary}</strong>
          </div>
          <div className="recipe-item">
            <span className="hint">Folder</span>
            <strong>{downloadFolder.trim() || 'Not set'}</strong>
          </div>
          <div className="recipe-item">
            <span className="hint">Filename</span>
            <strong>{downloadFileName.trim() || 'Suggested automatically'}</strong>
          </div>
          <div className="recipe-item recipe-item-wide">
            <span className="hint">Preview path</span>
            <strong>{previewPath}</strong>
          </div>
        </div>
      </div>
      {folderMissing ? (
        <div className="status-detail unavailable">
          <span className="status-badge danger">Action required</span>
          <p className="status-detail-text">
            Browser mode needs a destination folder before a transfer can start.
          </p>
        </div>
      ) : null}
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
          disabled={isReadOnly}
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
          disabled={isReadOnly}
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
              disabled={isReadOnly}
            />
          </label>
          <div className="row">
            <button type="button" onClick={onLoadHfFiles} disabled={isReadOnly}>
              Load files
            </button>
          </div>
          {hfFiles.length > 0 && (
            <label>
              Select file
              <select
                value={selectedHfFile}
                onChange={(e) => onSelectedHfFileChange(e.target.value)}
                disabled={isReadOnly}
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
            <button type="button" onClick={onLoadOllamaTags} disabled={isReadOnly}>
              Load tags
            </button>
          </div>
          {ollamaTags.length > 0 && (
            <label>
              Select tag
              <select
                value={selectedOllamaTag}
                onChange={(e) => onSelectedOllamaTagChange(e.target.value)}
                disabled={isReadOnly}
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
          disabled={isReadOnly}
        />
      </label>
      <button
        type="button"
        onClick={() => void onStartDownload()}
        disabled={!canStartDownload}
      >
        {startButtonLabel}
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
                  type="button"
                  onClick={() => void onCancelDownload(item.downloadId)}
                  disabled={isCancellingDownload(item.downloadId) || isReadOnly}
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
