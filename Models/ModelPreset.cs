namespace LlamaCppDesk.Models;

public sealed class ModelPreset
{
    public string Name { get; set; } = string.Empty;

    public string ModelPath { get; set; } = string.Empty;

    public string Host { get; set; } = "127.0.0.1";

    public int Port { get; set; } = 8080;

    public int ContextSize { get; set; } = 4096;

    public int Threads { get; set; } = Math.Max(2, Environment.ProcessorCount / 2);

    public int GpuLayers { get; set; } = 0;

    public float Temperature { get; set; } = 0.7f;

    public int MaxTokens { get; set; } = 512;
}
