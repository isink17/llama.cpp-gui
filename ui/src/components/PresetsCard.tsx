import type { Preset, Settings } from '../lib/tauri/api';

type PresetDraft = Preset;

type PresetsCardProps = {
  presets: Preset[];
  presetDraft: PresetDraft;
  settings: Settings;
  onNewPreset: () => void;
  onResetPresetDraft: () => void;
  onPresetDraftChange: (preset: PresetDraft) => void;
  onSavePresetFromSettings: () => void;
  onApplyPreset: (preset: Preset) => void;
  onEditPreset: (preset: Preset) => void;
  onDeletePreset: (presetId: string) => void;
  isSaving?: boolean;
  isDeletingPreset?: (presetId: string) => boolean;
};

export function PresetsCard({
  presets,
  presetDraft,
  onNewPreset,
  onResetPresetDraft,
  onPresetDraftChange,
  onSavePresetFromSettings,
  onApplyPreset,
  onEditPreset,
  onDeletePreset,
  isSaving = false,
  isDeletingPreset = () => false,
}: PresetsCardProps) {
  return (
    <article className="card">
      <div className="panel-header">
        <h2>Presets</h2>
        <button type="button" className="secondary" onClick={onNewPreset}>
          New preset
        </button>
      </div>
      <label>
        Preset name
        <input
          value={presetDraft.name}
          onChange={(e) =>
            onPresetDraftChange({ ...presetDraft, name: e.target.value })
          }
          placeholder="Name for this preset"
        />
      </label>
      <div className="row">
        <button type="button" onClick={onSavePresetFromSettings} disabled={isSaving}>
          {isSaving ? 'Saving...' : 'Save current settings as preset'}
        </button>
        <button type="button" className="secondary" onClick={onResetPresetDraft}>
          Clear
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
                </div>
                <div className="hint">
                  {preset.modelPath || 'No model path'} &middot;{' '}
                  {preset.host}:{preset.port} &middot; ctx={preset.contextSize}{' '}
                  t={preset.threads} gpu={preset.gpuLayers} &middot; temp=
                  {preset.temperature} max={preset.maxTokens}
                </div>
                <div className="row">
                  <button
                    type="button"
                    className="secondary"
                    onClick={() => onApplyPreset(preset)}
                    disabled={isDeletingPreset(preset.id)}
                  >
                    Apply
                  </button>
                  <button
                    type="button"
                    className="secondary"
                    onClick={() => onEditPreset(preset)}
                    disabled={isDeletingPreset(preset.id)}
                  >
                    Edit
                  </button>
                  <button
                    type="button"
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
