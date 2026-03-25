namespace LlamaCppDesk.Models;

public sealed class AppSettings
{
    public string LlamaServerPath { get; set; } = string.Empty;

    public string ModelPath { get; set; } = string.Empty;

    public string Host { get; set; } = "127.0.0.1";

    public int Port { get; set; } = 8080;

    public int ContextSize { get; set; } = 4096;

    public int Threads { get; set; } = Math.Max(2, Environment.ProcessorCount / 2);

    public int GpuLayers { get; set; } = 0;

    public float Temperature { get; set; } = 0.7f;

    public int MaxTokens { get; set; } = 512;

    public List<string> RecentModelPaths { get; set; } = new();

    public List<string> RecentServerPaths { get; set; } = new();

    public List<ModelPreset> Presets { get; set; } = new();

    public string DownloadFolder { get; set; } = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),
        "Downloads",
        "LLMModels");

    public List<string> RecentModelUrls { get; set; } = new();
}
