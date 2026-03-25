using System.Net.Http.Headers;
using System.Text.Json;

namespace LlamaCppDesk.Services;

public sealed class ModelReferenceResolverService
{
    private static readonly string[] PreferredQuantizations =
    [
        "Q4_K_M",
        "Q5_K_M",
        "Q4_0",
        "Q8_0"
    ];

    private readonly HttpClient _httpClient = new()
    {
        Timeout = TimeSpan.FromSeconds(30)
    };

    public async Task<ResolvedModelDownload> ResolveAsync(
        string source,
        string input,
        string? huggingFaceToken,
        CancellationToken cancellationToken = default)
    {
        var trimmedInput = input.Trim();
        if (string.IsNullOrWhiteSpace(trimmedInput))
        {
            throw new InvalidOperationException("Model reference is required.");
        }

        return source switch
        {
            "Direct URL" => ResolveDirectUrl(trimmedInput),
            "Hugging Face" => await ResolveHuggingFaceAsync(trimmedInput, huggingFaceToken, cancellationToken),
            "Ollama Library" => await ResolveOllamaAsync(trimmedInput, cancellationToken),
            _ => throw new InvalidOperationException($"Unsupported source: {source}")
        };
    }

    public async Task<IReadOnlyList<string>> ListHuggingFaceGgufFilesAsync(
        string input,
        string? token,
        CancellationToken cancellationToken = default)
    {
        ParseHuggingFaceReference(input, out var repoId, out _);
        var siblings = await FetchHuggingFaceSiblingsAsync(repoId, token, cancellationToken);
        return siblings
            .Where(static x => x.EndsWith(".gguf", StringComparison.OrdinalIgnoreCase))
            .OrderBy(static x => x, StringComparer.OrdinalIgnoreCase)
            .ToList();
    }

    public async Task<IReadOnlyList<string>> ListOllamaTagsAsync(
        string input,
        CancellationToken cancellationToken = default)
    {
        ParseOllamaReference(input, out var repoPath, out _);
        var endpoint = $"https://registry.ollama.ai/v2/{repoPath}/tags/list";

        using var response = await _httpClient.GetAsync(endpoint, cancellationToken);
        response.EnsureSuccessStatusCode();

        await using var stream = await response.Content.ReadAsStreamAsync(cancellationToken);
        using var doc = await JsonDocument.ParseAsync(stream, cancellationToken: cancellationToken);

        if (!doc.RootElement.TryGetProperty("tags", out var tagsElement) ||
            tagsElement.ValueKind != JsonValueKind.Array)
        {
            return [];
        }

        return tagsElement
            .EnumerateArray()
            .Select(static x => x.GetString())
            .Where(static x => !string.IsNullOrWhiteSpace(x))
            .Select(static x => x!)
            .OrderBy(static x => x, StringComparer.OrdinalIgnoreCase)
            .ToList();
    }

    private static ResolvedModelDownload ResolveDirectUrl(string input)
    {
        if (!Uri.TryCreate(input, UriKind.Absolute, out var uri))
        {
            throw new InvalidOperationException("Enter a valid absolute URL.");
        }

        var fileName = Path.GetFileName(uri.LocalPath);
        if (string.IsNullOrWhiteSpace(fileName))
        {
            fileName = "model.gguf";
        }

        return new ResolvedModelDownload(uri.ToString(), fileName);
    }

    private async Task<ResolvedModelDownload> ResolveHuggingFaceAsync(
        string input,
        string? token,
        CancellationToken cancellationToken)
    {
        if (Uri.TryCreate(input, UriKind.Absolute, out var directUri))
        {
            var directFileName = Path.GetFileName(directUri.LocalPath);
            if (string.IsNullOrWhiteSpace(directFileName))
            {
                directFileName = "model.gguf";
            }

            var directHeaders = BuildBearerAuthHeaders(token);
            return new ResolvedModelDownload(directUri.ToString(), directFileName, directHeaders);
        }

        ParseHuggingFaceReference(input, out var repoId, out var requestedFile);
        var siblings = await FetchHuggingFaceSiblingsAsync(repoId, token, cancellationToken);
        var ggufFiles = siblings
            .Where(static x => x.EndsWith(".gguf", StringComparison.OrdinalIgnoreCase))
            .ToList();

        if (ggufFiles.Count == 0)
        {
            throw new InvalidOperationException(
                $"No .gguf files found in Hugging Face repo '{repoId}'. Use 'repo::file.gguf' to select a file explicitly.");
        }

        string selectedFile;
        if (!string.IsNullOrWhiteSpace(requestedFile))
        {
            selectedFile = ggufFiles.FirstOrDefault(file =>
                    string.Equals(file, requestedFile, StringComparison.OrdinalIgnoreCase))
                ?? throw new InvalidOperationException(
                    $"File '{requestedFile}' was not found in '{repoId}'.");
        }
        else
        {
            selectedFile = PickBestGgufFile(ggufFiles);
        }

        var url = $"https://huggingface.co/{repoId}/resolve/main/{selectedFile}?download=true";
        var headers = BuildBearerAuthHeaders(token);
        return new ResolvedModelDownload(url, Path.GetFileName(selectedFile), headers);
    }

    private async Task<List<string>> FetchHuggingFaceSiblingsAsync(
        string repoId,
        string? token,
        CancellationToken cancellationToken)
    {
        var endpoint = $"https://huggingface.co/api/models/{repoId}";
        using var request = new HttpRequestMessage(HttpMethod.Get, endpoint);

        if (!string.IsNullOrWhiteSpace(token))
        {
            request.Headers.Authorization = new AuthenticationHeaderValue("Bearer", token.Trim());
        }

        using var response = await _httpClient.SendAsync(request, cancellationToken);
        response.EnsureSuccessStatusCode();

        await using var stream = await response.Content.ReadAsStreamAsync(cancellationToken);
        using var doc = await JsonDocument.ParseAsync(stream, cancellationToken: cancellationToken);

        if (!doc.RootElement.TryGetProperty("siblings", out var siblingsElement) ||
            siblingsElement.ValueKind != JsonValueKind.Array)
        {
            return [];
        }

        var results = new List<string>();
        foreach (var sibling in siblingsElement.EnumerateArray())
        {
            if (sibling.TryGetProperty("rfilename", out var fileElement))
            {
                var value = fileElement.GetString();
                if (!string.IsNullOrWhiteSpace(value))
                {
                    results.Add(value);
                }
            }
        }

        return results;
    }

    private static string PickBestGgufFile(IEnumerable<string> ggufFiles)
    {
        var ranked = ggufFiles
            .Select(file => new
            {
                File = file,
                Score = ScoreGgufFile(file)
            })
            .OrderByDescending(static x => x.Score)
            .ThenBy(static x => x.File.Length)
            .ToList();

        return ranked[0].File;
    }

    private static int ScoreGgufFile(string file)
    {
        var score = 0;
        for (var i = 0; i < PreferredQuantizations.Length; i++)
        {
            if (file.Contains(PreferredQuantizations[i], StringComparison.OrdinalIgnoreCase))
            {
                score += 100 - (i * 10);
            }
        }

        if (file.Contains("instruct", StringComparison.OrdinalIgnoreCase))
        {
            score += 5;
        }

        return score;
    }

    private static Dictionary<string, string>? BuildBearerAuthHeaders(string? token)
    {
        if (string.IsNullOrWhiteSpace(token))
        {
            return null;
        }

        return new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase)
        {
            ["Authorization"] = $"Bearer {token.Trim()}"
        };
    }

    private static void ParseHuggingFaceReference(string input, out string repoId, out string? file)
    {
        var parts = input.Split("::", 2, StringSplitOptions.TrimEntries);
        repoId = parts[0];
        file = parts.Length > 1 ? parts[1] : null;

        if (string.IsNullOrWhiteSpace(repoId) || !repoId.Contains('/', StringComparison.Ordinal))
        {
            throw new InvalidOperationException(
                "Hugging Face format: owner/repo or owner/repo::file.gguf");
        }
    }

    private async Task<ResolvedModelDownload> ResolveOllamaAsync(string input, CancellationToken cancellationToken)
    {
        ParseOllamaReference(input, out var repoPath, out var tag);
        var manifestUrl = $"https://registry.ollama.ai/v2/{repoPath}/manifests/{tag}";

        using var request = new HttpRequestMessage(HttpMethod.Get, manifestUrl);
        request.Headers.Accept.ParseAdd("application/vnd.docker.distribution.manifest.v2+json");
        request.Headers.Accept.ParseAdd("application/vnd.oci.image.manifest.v1+json");

        using var response = await _httpClient.SendAsync(request, cancellationToken);
        response.EnsureSuccessStatusCode();

        await using var stream = await response.Content.ReadAsStreamAsync(cancellationToken);
        using var doc = await JsonDocument.ParseAsync(stream, cancellationToken: cancellationToken);

        if (!doc.RootElement.TryGetProperty("layers", out var layersElement) ||
            layersElement.ValueKind != JsonValueKind.Array)
        {
            throw new InvalidOperationException("Unable to parse Ollama manifest layers.");
        }

        string? selectedDigest = null;
        foreach (var layer in layersElement.EnumerateArray())
        {
            var mediaType = layer.TryGetProperty("mediaType", out var mediaTypeElement)
                ? mediaTypeElement.GetString() ?? string.Empty
                : string.Empty;
            var digest = layer.TryGetProperty("digest", out var digestElement)
                ? digestElement.GetString()
                : null;

            if (string.IsNullOrWhiteSpace(digest))
            {
                continue;
            }

            if (mediaType.Contains("model", StringComparison.OrdinalIgnoreCase) ||
                mediaType.Contains("gguf", StringComparison.OrdinalIgnoreCase))
            {
                selectedDigest = digest;
                break;
            }

            selectedDigest ??= digest;
        }

        if (string.IsNullOrWhiteSpace(selectedDigest))
        {
            throw new InvalidOperationException("No downloadable model layer found in Ollama manifest.");
        }

        var downloadUrl = $"https://registry.ollama.ai/v2/{repoPath}/blobs/{selectedDigest}";
        var safeRepo = repoPath.Replace('/', '-');
        var safeDigest = selectedDigest.Replace(':', '-');
        var suggestedFile = $"{safeRepo}-{tag}-{safeDigest[..Math.Min(safeDigest.Length, 20)]}.gguf";

        return new ResolvedModelDownload(downloadUrl, suggestedFile);
    }

    private static void ParseOllamaReference(string input, out string repoPath, out string tag)
    {
        var normalized = input.Trim();
        if (normalized.StartsWith("ollama.com/library/", StringComparison.OrdinalIgnoreCase))
        {
            normalized = normalized["ollama.com/library/".Length..];
        }
        else if (normalized.StartsWith("https://ollama.com/library/", StringComparison.OrdinalIgnoreCase))
        {
            normalized = normalized["https://ollama.com/library/".Length..];
        }

        var colonIndex = normalized.LastIndexOf(':');
        if (colonIndex >= 0)
        {
            repoPath = normalized[..colonIndex];
            tag = normalized[(colonIndex + 1)..];
        }
        else
        {
            repoPath = normalized;
            tag = "latest";
        }

        if (string.IsNullOrWhiteSpace(repoPath))
        {
            throw new InvalidOperationException("Ollama format: model[:tag], for example 'llama3:8b'.");
        }

        if (!repoPath.Contains('/', StringComparison.Ordinal))
        {
            repoPath = "library/" + repoPath;
        }

        if (string.IsNullOrWhiteSpace(tag))
        {
            tag = "latest";
        }
    }
}

public sealed record ResolvedModelDownload(
    string DownloadUrl,
    string SuggestedFileName,
    IReadOnlyDictionary<string, string>? RequestHeaders = null);
