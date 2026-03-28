import type {
  LlamaProcessStatus,
  LlamaServerHealthStatus,
} from '../lib/tauri/api';
import { type FailureView, getFailureView } from '../utils/failure';

export type LlamaServerCardProps = {
  refreshIssue: string | null;
  processStatus: LlamaProcessStatus;
  processHealth: LlamaServerHealthStatus | null;
  serverUrl: string;
  isCheckingHealth: boolean;
  isProcessTransitionBusy: boolean;
  isStartingProcess: boolean;
  isRestartingProcess: boolean;
  isStoppingProcess: boolean;
  isClearingLogs: boolean;
  onCheckHealth: () => void;
  onStartProcess: () => void;
  onRestartProcess: () => void;
  onStopProcess: () => void;
  onClearLogs: () => void;
  llamaLogs: string[];
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
  isRestartingProcess,
  isStoppingProcess,
  isClearingLogs,
  onCheckHealth,
  onStartProcess,
  onRestartProcess,
  onStopProcess,
  onClearLogs,
  llamaLogs,
}: LlamaServerCardProps) {
  const refreshFailureView = refreshIssue ? getFailureView(refreshIssue) : null;
  const processHealthFailureView =
    processHealth && !processHealth.healthy && processHealth.message
      ? getFailureView(processHealth.message)
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
            type="button"
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
        <button
          type="button"
          onClick={() => void onStartProcess()}
          disabled={isProcessTransitionBusy || processStatus.running}
        >
          {isStartingProcess ? 'Starting...' : 'Start'}
        </button>
        <button
          type="button"
          onClick={() => void onRestartProcess()}
          disabled={isProcessTransitionBusy || !processStatus.running}
        >
          {isRestartingProcess ? 'Restarting...' : 'Restart'}
        </button>
        <button
          type="button"
          onClick={() => void onStopProcess()}
          disabled={isProcessTransitionBusy || !processStatus.running}
        >
          {isStoppingProcess ? 'Stopping...' : 'Shutdown'}
        </button>
      </div>
      <div className="panel">
        <div className="panel-header">
          <h3>Logs</h3>
          <button type="button" onClick={() => void onClearLogs()} disabled={isClearingLogs}>
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
