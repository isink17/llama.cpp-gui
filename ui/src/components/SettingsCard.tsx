import type { Settings } from '../lib/tauri/api';

export type SettingsCardProps = {
  settings: Settings;
  isSaving: boolean;
  onServerUrlChange: (value: string) => void;
  onMaxTokensChange: (value: number) => void;
  onTemperatureChange: (value: number) => void;
  onSave: () => void;
};

export function SettingsCard({
  settings,
  isSaving,
  onServerUrlChange,
  onMaxTokensChange,
  onTemperatureChange,
  onSave,
}: SettingsCardProps) {
  return (
    <article className="card">
      <h2>Settings</h2>
      <label>
        Server URL
        <input
          value={settings.serverUrl}
          onChange={(e) => onServerUrlChange(e.target.value)}
        />
      </label>
      <label>
        Max tokens
        <input
          type="number"
          value={settings.maxTokens}
          onChange={(e) => onMaxTokensChange(Number(e.target.value))}
        />
      </label>
      <label>
        Temperature
        <input
          type="number"
          step="0.1"
          value={settings.temperature}
          onChange={(e) => onTemperatureChange(Number(e.target.value))}
        />
      </label>
      <button onClick={() => void onSave()} disabled={isSaving}>
        {isSaving ? 'Saving...' : 'Save settings'}
      </button>
    </article>
  );
}
