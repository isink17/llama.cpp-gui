import type { Settings } from '../lib/tauri/api';

export type SettingsCardProps = {
  settings: Settings;
  isSaving: boolean;
  onChange: (patch: Partial<Settings>) => void;
  onSave: () => void;
  onBrowseServerPath: () => void;
  onBrowseModelPath: () => void;
  onBrowseDownloadFolder: () => void;
};

export function SettingsCard({
  settings,
  isSaving,
  onChange,
  onSave,
  onBrowseServerPath,
  onBrowseModelPath,
  onBrowseDownloadFolder,
}: SettingsCardProps) {
  return (
    <article className="card">
      <h2>Settings</h2>
      <label>
        llama-server path
        <div className="row">
          <input
            value={settings.llamaServerPath}
            onChange={(e) => onChange({ llamaServerPath: e.target.value })}
            placeholder="Path to llama-server executable"
          />
          <button type="button" onClick={onBrowseServerPath}>
            Browse...
          </button>
        </div>
      </label>
      <label>
        Model path
        <div className="row">
          <input
            value={settings.modelPath}
            onChange={(e) => onChange({ modelPath: e.target.value })}
            placeholder="Path to .gguf model file"
          />
          <button type="button" onClick={onBrowseModelPath}>
            Browse...
          </button>
        </div>
      </label>
      <div className="row">
        <label>
          Host
          <input
            value={settings.host}
            onChange={(e) => onChange({ host: e.target.value })}
          />
        </label>
        <label>
          Port
          <input
            type="number"
            value={settings.port}
            onChange={(e) => onChange({ port: Number(e.target.value) })}
          />
        </label>
      </div>
      <div className="row">
        <label>
          Context size
          <input
            type="number"
            value={settings.contextSize}
            onChange={(e) => onChange({ contextSize: Number(e.target.value) })}
          />
        </label>
        <label>
          Threads
          <input
            type="number"
            value={settings.threads}
            onChange={(e) => onChange({ threads: Number(e.target.value) })}
          />
        </label>
        <label>
          GPU layers
          <input
            type="number"
            value={settings.gpuLayers}
            onChange={(e) => onChange({ gpuLayers: Number(e.target.value) })}
          />
        </label>
      </div>
      <div className="row">
        <label>
          Temperature
          <input
            type="number"
            step="0.1"
            value={settings.temperature}
            onChange={(e) => onChange({ temperature: Number(e.target.value) })}
          />
        </label>
        <label>
          Max tokens
          <input
            type="number"
            value={settings.maxTokens}
            onChange={(e) => onChange({ maxTokens: Number(e.target.value) })}
          />
        </label>
      </div>
      <label>
        Download folder
        <div className="row">
          <input
            value={settings.downloadFolder}
            onChange={(e) => onChange({ downloadFolder: e.target.value })}
            placeholder="Folder for downloaded models"
          />
          <button type="button" onClick={onBrowseDownloadFolder}>
            Browse...
          </button>
        </div>
      </label>
      <button onClick={() => void onSave()} disabled={isSaving}>
        {isSaving ? 'Saving...' : 'Save settings'}
      </button>
    </article>
  );
}
