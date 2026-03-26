import type { Preset } from '../lib/tauri/api';

type PresetDraft = Preset;

type PresetsCardProps = {
  presets: Preset[];
  presetDraft: PresetDraft;
  onNewPreset: () => void;
  onResetPresetDraft: () => void;
  onPresetDraftChange: (preset: PresetDraft) => void;
  onSavePreset: () => void;
  onEditPreset: (preset: Preset) => void;
  onDeletePreset: (presetId: string) => void;
  formatTimestamp?: (value: string) => string;
  isSaving?: boolean;
  isDeletingPreset?: (presetId: string) => boolean;
};

const defaultFormatTimestamp = (value: string) => {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString();
};

export function PresetsCard({
  presets,
  presetDraft,
  onNewPreset,
  onResetPresetDraft,
  onPresetDraftChange,
  onSavePreset,
  onEditPreset,
  onDeletePreset,
  formatTimestamp = defaultFormatTimestamp,
  isSaving = false,
  isDeletingPreset = () => false,
}: PresetsCardProps) {
  return (
    <article className="card">
      <div className="panel-header">
        <h2>Presets</h2>
        <button className="secondary" onClick={onNewPreset}>
          New preset
        </button>
      </div>
      <label>
        Name
        <input
          value={presetDraft.name}
          onChange={(e) =>
            onPresetDraftChange({ ...presetDraft, name: e.target.value })
          }
        />
      </label>
      <label>
        Preset ID
        <input
          value={presetDraft.id}
          onChange={(e) =>
            onPresetDraftChange({ ...presetDraft, id: e.target.value })
          }
          placeholder="Generated automatically for new presets"
        />
      </label>
      <label>
        System prompt
        <textarea
          value={presetDraft.systemPrompt}
          onChange={(e) =>
            onPresetDraftChange({ ...presetDraft, systemPrompt: e.target.value })
          }
          rows={4}
        />
      </label>
      <div className="row">
        <button onClick={onSavePreset} disabled={isSaving}>
          {isSaving ? 'Saving...' : 'Save preset'}
        </button>
        <button className="secondary" onClick={onResetPresetDraft}>
          Clear form
        </button>
      </div>
      <div className="panel">
        <div className="panel-header">
          <h3>Saved presets</h3>
          <span className="hint">{presets.length} items</span>
        </div>
        <div className="entry-list">
          {presets.length ? (
            presets.map((preset) => (
              <article className="entry-card" key={preset.id}>
                <div className="entry-card-header">
                  <div>
                    <strong>{preset.name}</strong>
                    <div className="hint">{preset.id}</div>
                  </div>
                  <div className="hint">{formatTimestamp(preset.createdAt)}</div>
                </div>
                <p>{preset.systemPrompt || 'No system prompt stored.'}</p>
                <div className="row">
                  <button
                    className="secondary"
                    onClick={() => onEditPreset(preset)}
                    disabled={isDeletingPreset(preset.id)}
                  >
                    Edit
                  </button>
                  <button
                    className="danger"
                    onClick={() => onDeletePreset(preset.id)}
                    disabled={isDeletingPreset(preset.id)}
                  >
                    {isDeletingPreset(preset.id) ? 'Deleting...' : 'Delete'}
                  </button>
                </div>
              </article>
            ))
          ) : (
            <p className="empty-state">No presets saved yet.</p>
          )}
        </div>
      </div>
    </article>
  );
}
