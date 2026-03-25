using System.Diagnostics;
using System.Net.Http.Json;
using System.Runtime.CompilerServices;
using System.Text;
using System.Text.Json;
using LlamaCppDesk.Models;

namespace LlamaCppDesk.Services;

public sealed class LlamaServerService : IDisposable
{
    private readonly HttpClient _httpClient;
    private Process? _process;

    public LlamaServerService()
    {
        _httpClient = new HttpClient
        {
            Timeout = TimeSpan.FromMinutes(3)
        };
    }

    public bool IsRunning => _process is { HasExited: false };

    public async Task StartAsync(AppSettings settings, CancellationToken cancellationToken = default)
    {
        if (IsRunning)
        {
            return;
        }

        if (string.IsNullOrWhiteSpace(settings.LlamaServerPath) || !File.Exists(settings.LlamaServerPath))
        {
            throw new FileNotFoundException("llama-server executable not found.", settings.LlamaServerPath);
        }

        if (string.IsNullOrWhiteSpace(settings.ModelPath) || !File.Exists(settings.ModelPath))
        {
            throw new FileNotFoundException("Model file not found.", settings.ModelPath);
        }

        var args = BuildArguments(settings);
        var startInfo = new ProcessStartInfo
        {
            FileName = settings.LlamaServerPath,
            Arguments = args,
            UseShellExecute = false,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            CreateNoWindow = true,
            WorkingDirectory = Path.GetDirectoryName(settings.LlamaServerPath) ?? Environment.CurrentDirectory
        };

        _process = new Process
        {
            StartInfo = startInfo,
            EnableRaisingEvents = true
        };

        _process.Start();
        _process.BeginOutputReadLine();
        _process.BeginErrorReadLine();

        await WaitForReadyAsync(settings, cancellationToken);
    }

    public async Task StopAsync(CancellationToken cancellationToken = default)
    {
        if (_process is null || _process.HasExited)
        {
            return;
        }

        try
        {
            _process.Kill(entireProcessTree: true);
            await _process.WaitForExitAsync(cancellationToken);
        }
        catch (InvalidOperationException)
        {
            // Process already exited.
        }
    }

    public async Task<string> ChatAsync(AppSettings settings, IReadOnlyList<ChatMessage> messages, CancellationToken cancellationToken = default)
    {
        if (messages.Count == 0)
        {
            return "";
        }

        var endpoint = BuildServerUrl(settings) + "/v1/chat/completions";

        var payload = new
        {
            model = "llama",
            messages = messages.Select(static m => new { role = m.Role, content = m.Content }).ToArray(),
            stream = false,
            temperature = settings.Temperature,
            max_tokens = settings.MaxTokens
        };

        using var response = await _httpClient.PostAsJsonAsync(endpoint, payload, cancellationToken);
        response.EnsureSuccessStatusCode();

        await using var stream = await response.Content.ReadAsStreamAsync(cancellationToken);
        using var document = await JsonDocument.ParseAsync(stream, cancellationToken: cancellationToken);

        if (document.RootElement.TryGetProperty("choices", out var choices) &&
            choices.ValueKind == JsonValueKind.Array &&
            choices.GetArrayLength() > 0)
        {
            var choice = choices[0];
            if (choice.TryGetProperty("message", out var message) &&
                message.TryGetProperty("content", out var contentElement))
            {
                return contentElement.GetString() ?? string.Empty;
            }
        }

        return "No response from model.";
    }

    public async IAsyncEnumerable<string> StreamChatAsync(
        AppSettings settings,
        IReadOnlyList<ChatMessage> messages,
        [EnumeratorCancellation] CancellationToken cancellationToken = default)
    {
        if (messages.Count == 0)
        {
            yield break;
        }

        var endpoint = BuildServerUrl(settings) + "/v1/chat/completions";
        var payload = new
        {
            model = "llama",
            messages = messages.Select(static m => new { role = m.Role, content = m.Content }).ToArray(),
            stream = true,
            temperature = settings.Temperature,
            max_tokens = settings.MaxTokens
        };

        using var request = new HttpRequestMessage(HttpMethod.Post, endpoint)
        {
            Content = JsonContent.Create(payload)
        };

        using var response = await _httpClient.SendAsync(
            request,
            HttpCompletionOption.ResponseHeadersRead,
            cancellationToken);
        response.EnsureSuccessStatusCode();

        await using var stream = await response.Content.ReadAsStreamAsync(cancellationToken);
        using var reader = new StreamReader(stream, Encoding.UTF8);

        while (true)
        {
            cancellationToken.ThrowIfCancellationRequested();

            var line = await reader.ReadLineAsync();
            if (line is null)
            {
                break;
            }

            if (string.IsNullOrWhiteSpace(line) || !line.StartsWith("data:", StringComparison.Ordinal))
            {
                continue;
            }

            var data = line["data:".Length..].Trim();
            if (data.Equals("[DONE]", StringComparison.Ordinal))
            {
                break;
            }

            var token = ExtractDeltaToken(data);
            if (!string.IsNullOrEmpty(token))
            {
                yield return token;
            }
        }
    }

    private static string BuildArguments(AppSettings settings)
    {
        var args = new List<string>
        {
            "--host", Quote(settings.Host),
            "--port", settings.Port.ToString(),
            "-m", Quote(settings.ModelPath),
            "-c", settings.ContextSize.ToString(),
            "-t", settings.Threads.ToString()
        };

        if (settings.GpuLayers > 0)
        {
            args.Add("--n-gpu-layers");
            args.Add(settings.GpuLayers.ToString());
        }

        return string.Join(" ", args);
    }

    private static string Quote(string value)
        => $"\"{value.Replace("\"", "\\\"")}\"";

    private static string BuildServerUrl(AppSettings settings)
        => $"http://{settings.Host}:{settings.Port}";

    private static string ExtractDeltaToken(string json)
    {
        try
        {
            using var document = JsonDocument.Parse(json);
            var root = document.RootElement;
            if (!root.TryGetProperty("choices", out var choices) ||
                choices.ValueKind != JsonValueKind.Array ||
                choices.GetArrayLength() == 0)
            {
                return string.Empty;
            }

            var firstChoice = choices[0];
            if (firstChoice.TryGetProperty("delta", out var delta) &&
                delta.TryGetProperty("content", out var deltaContent))
            {
                return deltaContent.GetString() ?? string.Empty;
            }

            if (firstChoice.TryGetProperty("message", out var message) &&
                message.TryGetProperty("content", out var messageContent))
            {
                return messageContent.GetString() ?? string.Empty;
            }

            if (firstChoice.TryGetProperty("text", out var text))
            {
                return text.GetString() ?? string.Empty;
            }
        }
        catch
        {
            // Ignore malformed chunks and continue consuming the stream.
        }

        return string.Empty;
    }

    private async Task WaitForReadyAsync(AppSettings settings, CancellationToken cancellationToken)
    {
        var healthUrl = BuildServerUrl(settings) + "/health";
        var modelsUrl = BuildServerUrl(settings) + "/v1/models";

        using var timeoutCts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        timeoutCts.CancelAfter(TimeSpan.FromSeconds(45));

        while (!timeoutCts.IsCancellationRequested)
        {
            if (_process is { HasExited: true })
            {
                throw new InvalidOperationException("llama-server exited before becoming ready.");
            }

            if (await IsHealthyAsync(healthUrl, timeoutCts.Token) || await IsHealthyAsync(modelsUrl, timeoutCts.Token))
            {
                return;
            }

            await Task.Delay(600, timeoutCts.Token);
        }

        throw new TimeoutException("Timed out waiting for llama-server to be ready.");
    }

    private async Task<bool> IsHealthyAsync(string url, CancellationToken cancellationToken)
    {
        try
        {
            using var response = await _httpClient.GetAsync(url, cancellationToken);
            return response.IsSuccessStatusCode;
        }
        catch
        {
            return false;
        }
    }

    public void Dispose()
    {
        _httpClient.Dispose();
        _process?.Dispose();
    }
}
