export type RemoteApiCardProps = {
  baseUrl: string;
  token: string;
  isSaving: boolean;
  isConfigured: boolean;
  baseUrlSourceLabel: string;
  tokenSourceLabel: string;
  onBaseUrlChange: (value: string) => void;
  onTokenChange: (value: string) => void;
  onSave: () => void;
  onClear: () => void;
};

export function RemoteApiCard({
  baseUrl,
  token,
  isSaving,
  isConfigured,
  baseUrlSourceLabel,
  tokenSourceLabel,
  onBaseUrlChange,
  onTokenChange,
  onSave,
  onClear,
}: RemoteApiCardProps) {
  const hasValidConfig = isConfigured && baseUrl.trim().length > 0 && token.trim().length > 0;

  return (
    <article className="card remote-api-card">
      <div className="panel-header remote-api-card__header">
        <div>
          <p className="card-kicker">Browser transport</p>
          <h2>Remote API</h2>
        </div>
        <span className={`status-badge ${hasValidConfig ? 'success' : 'neutral'}`}>
          {hasValidConfig ? 'Connected' : 'Disconnected'}
        </span>
      </div>
      <p className="card-copy">
        Save the remote backend address and bearer token. The browser keeps them
        in local storage and uses them for HTTP requests plus the chat event stream.
      </p>
      <div className="remote-api-summary">
        <div>
          <span className="hint">Base URL</span>
          <strong>{baseUrl.trim() || 'Not set'}</strong>
          <div className="remote-api-source">{baseUrlSourceLabel}</div>
        </div>
        <div>
          <span className="hint">Token</span>
          <strong>{token.trim() ? 'Configured' : 'Not set'}</strong>
          <div className="remote-api-source">{tokenSourceLabel}</div>
        </div>
      </div>
      <label>
        Base URL
        <input
          value={baseUrl}
          onChange={(e) => onBaseUrlChange(e.target.value)}
          placeholder="http://127.0.0.1:8080"
          autoComplete="off"
        />
      </label>
      <div className="remote-api-hint">
        Example: <code>http://127.0.0.1:8080</code>
      </div>
      <label>
        Bearer token
        <input
          type="password"
          value={token}
          onChange={(e) => onTokenChange(e.target.value)}
          placeholder="Bearer token"
          autoComplete="off"
        />
      </label>
      <div className="remote-api-hint">
        Use the bearer token configured on the remote Tauri backend.
      </div>
      <div className="row">
        <button type="button" onClick={() => void onSave()} disabled={isSaving}>
          {isSaving ? 'Saving...' : 'Save remote config'}
        </button>
        <button type="button" className="secondary" onClick={onClear} disabled={isSaving}>
          Clear
        </button>
      </div>
    </article>
  );
}
