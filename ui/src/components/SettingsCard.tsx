import type { Settings } from '../lib/tauri/api';

export type SettingsCardProps = {
  settings: Settings;
  isSaving: boolean;
  canUseBackend: boolean;
  canBrowseFiles: boolean;
  serverPathSourceLabel: string;
  modelPathSourceLabel: string;
  downloadFolderSourceLabel: string;
  onChange: (patch: Partial<Settings>) => void;
  onSave: () => void;
  onBrowseServerPath: () => void;
  onBrowseModelPath: () => void;
  onBrowseDownloadFolder: () => void;
};

export function SettingsCard({
  settings,
  isSaving,
  canUseBackend,
  canBrowseFiles,
  serverPathSourceLabel,
  modelPathSourceLabel,
  downloadFolderSourceLabel,
  onChange,
  onSave,
  onBrowseServerPath,
  onBrowseModelPath,
  onBrowseDownloadFolder,
}: SettingsCardProps) {
  const isReadOnly = !canUseBackend;

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
            disabled={isReadOnly}
          />
          <button type="button" onClick={onBrowseServerPath} disabled={isReadOnly || !canBrowseFiles}>
            Browse...
          </button>
        </div>
        <div className="field-source">{serverPathSourceLabel}</div>
      </label>
      <label>
        Model path
        <div className="row">
          <input
            value={settings.modelPath}
            onChange={(e) => onChange({ modelPath: e.target.value })}
            placeholder="Path to .gguf model file"
            disabled={isReadOnly}
          />
          <button type="button" onClick={onBrowseModelPath} disabled={isReadOnly || !canBrowseFiles}>
            Browse...
          </button>
        </div>
        <div className="field-source">{modelPathSourceLabel}</div>
      </label>
      <div className="row">
        <label>
          Host
          <input
            value={settings.host}
            onChange={(e) => onChange({ host: e.target.value })}
            disabled={isReadOnly}
          />
        </label>
        <label>
          Port
          <input
            type="number"
            value={settings.port}
            onChange={(e) => onChange({ port: Number(e.target.value) })}
            disabled={isReadOnly}
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
            disabled={isReadOnly}
          />
        </label>
        <label>
          Threads
          <input
            type="number"
            value={settings.threads}
            onChange={(e) => onChange({ threads: Number(e.target.value) })}
            disabled={isReadOnly}
          />
        </label>
        <label>
          GPU layers
          <input
            type="number"
            value={settings.gpuLayers}
            onChange={(e) => onChange({ gpuLayers: Number(e.target.value) })}
            disabled={isReadOnly}
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
            disabled={isReadOnly}
          />
        </label>
        <label>
          Max tokens
          <input
            type="number"
            value={settings.maxTokens}
            onChange={(e) => onChange({ maxTokens: Number(e.target.value) })}
            disabled={isReadOnly}
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
            disabled={isReadOnly}
          />
          <button
            type="button"
            onClick={onBrowseDownloadFolder}
            disabled={isReadOnly || !canBrowseFiles}
          >
            Browse...
          </button>
        </div>
        <div className="field-source">{downloadFolderSourceLabel}</div>
      </label>
      <button
        type="button"
        onClick={() => void onSave()}
        disabled={isSaving || isReadOnly}
      >
        {isSaving ? 'Saving...' : 'Save settings'}
      </button>
    </article>
  );
}
