import type {
  LlamaProcessStatus,
  LlamaServerHealthStatus,
} from '../lib/tauri/api';

type FailureKind = 'timeout' | 'unavailable' | 'failure';

type FailureView = {
  kind: FailureKind;
  label: string;
  tone: 'info' | 'danger';
  detailClass: 'timeout' | 'unavailable' | 'failure';
};

export type LlamaServerCardProps = {
  refreshIssue: string | null;
  processStatus: LlamaProcessStatus;
  processHealth: LlamaServerHealthStatus | null;
  serverUrl: string;
  isCheckingHealth: boolean;
  isProcessTransitionBusy: boolean;
  isStartingProcess: boolean;
  isStoppingProcess: boolean;
  isClearingLogs: boolean;
  onCheckHealth: () => void;
  onStartProcess: () => void;
  onStopProcess: () => void;
  onClearLogs: () => void;
  llamaLogs: string[];
};

const getFailureLabel = (kind: FailureKind) => {
  switch (kind) {
    case 'timeout':
      return 'Timeout';
    case 'unavailable':
      return 'Unavailable';
    case 'failure':
      return 'Failure';
  }
};

const getFailureTone = (kind: FailureKind): FailureView['tone'] => {
  switch (kind) {
    case 'timeout':
      return 'info';
    case 'unavailable':
    case 'failure':
      return 'danger';
  }
};

const getFailureDetailClass = (kind: FailureKind): FailureView['detailClass'] => {
  switch (kind) {
    case 'timeout':
      return 'timeout';
    case 'unavailable':
      return 'unavailable';
    case 'failure':
      return 'failure';
  }
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

const buildFailureView = (message: string): FailureView => {
  const kind = classifyFailureMessage(message);

  return {
    kind,
    label: getFailureLabel(kind),
    tone: getFailureTone(kind),
    detailClass: getFailureDetailClass(kind),
  };
};

const getHealthTone = (healthy: boolean | null) => {
  if (healthy === null) {
    return 'idle';
  }

  return healthy ? 'healthy' : 'unhealthy';
};

const getProcessBadgeLabel = (processStatus: LlamaProcessStatus) => {
  if (processStatus.running) {
    return 'Running';
  }

  if (
    processStatus.lastExitCode !== undefined &&
    processStatus.lastExitCode !== null
  ) {
    return `Exited ${processStatus.lastExitCode}`;
  }

  return 'Stopped';
};

const getProcessBadgeTone = (processStatus: LlamaProcessStatus) => {
  if (processStatus.running) {
    return 'success';
  }

  if (
    processStatus.lastExitCode !== undefined &&
    processStatus.lastExitCode !== null
  ) {
    return 'danger';
  }

  return 'neutral';
};

export function LlamaServerCard({
  refreshIssue,
  processStatus,
  processHealth,
  serverUrl,
  isCheckingHealth,
  isProcessTransitionBusy,
  isStartingProcess,
  isStoppingProcess,
  isClearingLogs,
  onCheckHealth,
  onStartProcess,
  onStopProcess,
  onClearLogs,
  llamaLogs,
}: LlamaServerCardProps) {
  const refreshFailureView = refreshIssue ? buildFailureView(refreshIssue) : null;
  const processHealthFailureView =
    processHealth && !processHealth.healthy && processHealth.message
      ? buildFailureView(processHealth.message)
      : null;
  const processHealthLabel = processHealth
    ? processHealth.healthy
      ? 'Healthy'
      : 'Unhealthy'
    : 'Not checked';
  const processHealthClass = getHealthTone(processHealth?.healthy ?? null);
  const processHealthDetailTone = processHealth?.healthy
    ? 'success'
    : processHealthFailureView
      ? processHealthFailureView.tone
      : 'danger';

  return (
    <article className="card">
      <h2>llama-server</h2>
      <div className="status-summary">
        <span className={`status-badge ${refreshFailureView?.tone ?? 'success'}`}>
          {refreshIssue ? 'Refresh failed' : 'Sync live'}
        </span>
        <span className="hint">
          {refreshIssue ? refreshIssue : 'Auto-refresh polling is active.'}
        </span>
      </div>
      {refreshFailureView ? (
        <div className={`status-detail ${refreshFailureView.detailClass}`}>
          <span className={`status-badge ${refreshFailureView.tone}`}>
            {refreshFailureView.label}
          </span>
          <p className="status-detail-text">{refreshIssue}</p>
        </div>
      ) : null}
      <div className="status-summary">
        <span className={`status-badge ${getProcessBadgeTone(processStatus)}`}>
          {getProcessBadgeLabel(processStatus)}
        </span>
        <span className="hint">PID {processStatus.pid ?? '-'}</span>
      </div>
      <div className="health-panel">
        <div className="panel-header">
          <h3>Health</h3>
          <button
            className="secondary"
            onClick={() => void onCheckHealth()}
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
          <div
            className={`status-detail ${
              processHealthFailureView?.detailClass ?? 'failure'
            }`}
          >
            <span className={`status-badge ${processHealthDetailTone}`}>
              {processHealth.healthy
                ? 'Service responding'
                : processHealthFailureView?.label ?? 'Failure'}
            </span>
            <p className="status-detail-text">{processHealth.message}</p>
          </div>
        ) : null}
      </div>
      <div className="hint">Server: {serverUrl}</div>
      <div className="row">
        <button onClick={() => void onStartProcess()} disabled={isProcessTransitionBusy}>
          {isStartingProcess ? 'Starting...' : 'Start'}
        </button>
        <button onClick={() => void onStopProcess()} disabled={isProcessTransitionBusy}>
          {isStoppingProcess ? 'Stopping...' : 'Stop'}
        </button>
      </div>
      <div className="panel">
        <div className="panel-header">
          <h3>Logs</h3>
          <button onClick={() => void onClearLogs()} disabled={isClearingLogs}>
            {isClearingLogs ? 'Clearing...' : 'Clear logs'}
          </button>
        </div>
        <pre className="log-panel">
          {llamaLogs.length ? llamaLogs.join('\n') : 'No llama-server logs yet.'}
        </pre>
      </div>
    </article>
  );
}
