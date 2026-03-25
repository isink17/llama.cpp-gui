using System.Text.Json;
using LlamaCppDesk.Models;

namespace LlamaCppDesk.Services;

public static class SettingsStore
{
    private const string SettingsFileName = "settings.json";

    public static async Task<AppSettings> LoadAsync(CancellationToken cancellationToken = default)
    {
        var path = GetSettingsFilePath();
        if (!File.Exists(path))
        {
            return new AppSettings();
        }

        await using var stream = File.OpenRead(path);
        var settings = await JsonSerializer.DeserializeAsync<AppSettings>(stream, cancellationToken: cancellationToken);
        return settings ?? new AppSettings();
    }

    public static async Task SaveAsync(AppSettings settings, CancellationToken cancellationToken = default)
    {
        var folder = GetSettingsFolderPath();
        Directory.CreateDirectory(folder);

        var path = GetSettingsFilePath();
        await using var stream = File.Create(path);
        await JsonSerializer.SerializeAsync(stream, settings, new JsonSerializerOptions { WriteIndented = true }, cancellationToken);
    }

    public static string GetSettingsFolderPath()
    {
        var appData = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
        return Path.Combine(appData, "LlamaCppDesk");
    }

    public static string GetSettingsFilePath()
        => Path.Combine(GetSettingsFolderPath(), SettingsFileName);
}