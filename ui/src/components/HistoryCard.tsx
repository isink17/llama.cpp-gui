import type { HistoryEntry, HistoryRole } from '../lib/tauri/api';

type HistoryCardProps = {
  historyEntries: HistoryEntry[];
  historyRole: HistoryRole;
  historyContent: string;
  onRoleChange: (role: HistoryRole) => void;
  onContentChange: (content: string) => void;
  onClearHistory: () => void;
  onAppendHistoryEntry: () => void;
  formatTimestamp?: (value: string) => string;
  isClearing?: boolean;
  isAppending?: boolean;
};

const HISTORY_ROLES: HistoryRole[] = ['system', 'user', 'assistant'];

const defaultFormatTimestamp = (value: string) => {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString();
};

export function HistoryCard({
  historyEntries,
  historyRole,
  historyContent,
  onRoleChange,
  onContentChange,
  onClearHistory,
  onAppendHistoryEntry,
  formatTimestamp = defaultFormatTimestamp,
  isClearing = false,
  isAppending = false,
}: HistoryCardProps) {
  return (
    <article className="card">
      <div className="panel-header">
        <h2>History</h2>
        <button
          type="button"
          className="secondary"
          onClick={onClearHistory}
          disabled={isClearing}
        >
          {isClearing ? 'Clearing...' : 'Clear history'}
        </button>
      </div>
      <div className="field-grid">
        <label>
          Role
          <select
            value={historyRole}
            onChange={(e) => onRoleChange(e.target.value as HistoryRole)}
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
          onChange={(e) => onContentChange(e.target.value)}
          rows={4}
          placeholder="Append a history entry manually."
        />
      </label>
      <div className="row">
        <button type="button" onClick={onAppendHistoryEntry} disabled={isAppending}>
          {isAppending ? 'Appending...' : 'Append history'}
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
  );
}
