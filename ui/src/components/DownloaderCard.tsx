import type { DownloadStatus } from '../lib/tauri/api';

type FailureKind = 'timeout' | 'unavailable' | 'failure';

type FailureView = {
  label: string;
  tone: 'info' | 'danger';
  detailClass: FailureKind;
};

export type DownloaderCardProps = {
  downloads: DownloadStatus[];
  downloadUrl: string;
  downloadPath: string;
  isStartingDownload: boolean;
  isCancellingDownload: (downloadId: string) => boolean;
  onDownloadUrlChange: (value: string) => void;
  onDownloadPathChange: (value: string) => void;
  onStartDownload: () => void;
  onCancelDownload: (downloadId: string) => void;
};

const classifyFailureMessage = (message: string): FailureKind => {
  const normalized = message.trim().toLowerCase();

  if (normalized.includes('timed out') || normalized.includes('timeout')) {
    return 'timeout';
  }

  if (
    normalized.includes('unavailable') ||
    normalized.includes('connection refused') ||
    normalized.includes('failed to reach')
  ) {
    return 'unavailable';
  }

  return 'failure';
};

const getFailureView = (message: string): FailureView => {
  const kind = classifyFailureMessage(message);

  return {
    label:
      kind === 'timeout' ? 'Timeout' : kind === 'unavailable' ? 'Unavailable' : 'Failure',
    tone: kind === 'timeout' ? 'info' : 'danger',
    detailClass: kind,
  };
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
  downloadPath,
  isStartingDownload,
  isCancellingDownload,
  onDownloadUrlChange,
  onDownloadPathChange,
  onStartDownload,
  onCancelDownload,
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
        Source URL
        <input value={downloadUrl} onChange={(e) => onDownloadUrlChange(e.target.value)} />
      </label>
      <label>
        Destination path
        <input value={downloadPath} onChange={(e) => onDownloadPathChange(e.target.value)} />
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
