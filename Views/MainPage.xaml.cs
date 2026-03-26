using System.Collections.ObjectModel;
using System.Collections.Specialized;
using System.ComponentModel;
using System.Runtime.CompilerServices;
using LlamaCppDesk.Models;
using LlamaCppDesk.Services;
using Microsoft.UI;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Windows.ApplicationModel.DataTransfer;
using Windows.Storage.Pickers;
using WinRT.Interop;

namespace LlamaCppDesk.Views;

public sealed partial class MainPage : Page, INotifyPropertyChanged
{
    private const int MaxHistoryEntries = 12;
    private const int MaxPromptHistoryEntries = 40;

    private readonly LlamaServerService _serverService = new();
    private readonly ModelDownloadService _downloadService = new();
    private readonly ModelReferenceResolverService _modelReferenceResolver = new();
    private AppSettings _settings = new();
    private CancellationTokenSource? _downloadCts;
    private CancellationTokenSource? _chatCts;

    private string _serverStatusText = "Server is stopped.";
    private string _headerStatusText = "Offline";
    private string _downloadProgressText = "Idle.";
    private string _selectedDownloadSource = "Direct URL";
    private string _downloadInputLabel = "Model URL (.gguf direct link)";
    private string _downloadInputPlaceholder = "https://example.com/model.gguf";
    private string _downloadSourceHintText = "Paste a direct link to a .gguf model file.";
    private string _huggingFaceFilesStatusText = "Load .gguf file list from repo.";
    private string? _selectedHuggingFaceGgufFile;
    private string _ollamaTagsStatusText = "Load tags from Ollama registry.";
    private string? _selectedOllamaTag;
    private string _composerHintText = "Enter send, Shift+Enter new line, Ctrl+Up/Down history, /help commands.";
    private Brush _statusDotBrush = new SolidColorBrush(Colors.IndianRed);
    private bool _isChatBusy;
    private Visibility _emptyStateVisibility = Visibility.Visible;
    private readonly List<string> _promptHistory = [];
    private int _promptHistoryIndex = -1;
    private string _promptDraftBeforeHistory = string.Empty;

    public ObservableCollection<ChatMessage> Messages { get; } = new();

    public ObservableCollection<string> ModelHistory { get; } = new();

    public ObservableCollection<string> ServerHistory { get; } = new();

    public ObservableCollection<string> PresetNames { get; } = new();

    public ObservableCollection<string> DownloadUrlHistory { get; } = new();
    public ObservableCollection<string> DownloadSourceOptions { get; } =
    [
        "Direct URL",
        "Hugging Face",
        "Ollama Library"
    ];
    public ObservableCollection<string> HuggingFaceGgufFiles { get; } = new();
    public ObservableCollection<string> OllamaTags { get; } = new();

    public string ServerStatusText
    {
        get => _serverStatusText;
        set => SetProperty(ref _serverStatusText, value);
    }

    public string HeaderStatusText
    {
        get => _headerStatusText;
        set => SetProperty(ref _headerStatusText, value);
    }

    public Brush StatusDotBrush
    {
        get => _statusDotBrush;
        set => SetProperty(ref _statusDotBrush, value);
    }

    public string DownloadProgressText
    {
        get => _downloadProgressText;
        set => SetProperty(ref _downloadProgressText, value);
    }

    public string SelectedDownloadSource
    {
        get => _selectedDownloadSource;
        set
        {
            if (!SetProperty(ref _selectedDownloadSource, value))
            {
                return;
            }

            UpdateDownloadSourceUi();
        }
    }

    public string DownloadInputLabel
    {
        get => _downloadInputLabel;
        set => SetProperty(ref _downloadInputLabel, value);
    }

    public string DownloadInputPlaceholder
    {
        get => _downloadInputPlaceholder;
        set => SetProperty(ref _downloadInputPlaceholder, value);
    }

    public string DownloadSourceHintText
    {
        get => _downloadSourceHintText;
        set => SetProperty(ref _downloadSourceHintText, value);
    }

    public string HuggingFaceFilesStatusText
    {
        get => _huggingFaceFilesStatusText;
        set => SetProperty(ref _huggingFaceFilesStatusText, value);
    }

    public string? SelectedHuggingFaceGgufFile
    {
        get => _selectedHuggingFaceGgufFile;
        set
        {
            if (!SetProperty(ref _selectedHuggingFaceGgufFile, value))
            {
                return;
            }

            if (string.IsNullOrWhiteSpace(value) ||
                !TryGetHuggingFaceRepoId(DownloadUrlBox.Text, out var repoId))
            {
                return;
            }

            DownloadUrlBox.Text = $"{repoId}::{value}";
        }
    }

    public string OllamaTagsStatusText
    {
        get => _ollamaTagsStatusText;
        set => SetProperty(ref _ollamaTagsStatusText, value);
    }

    public string? SelectedOllamaTag
    {
        get => _selectedOllamaTag;
        set
        {
            if (!SetProperty(ref _selectedOllamaTag, value))
            {
                return;
            }

            if (string.IsNullOrWhiteSpace(value) ||
                !TryGetOllamaModelName(DownloadUrlBox.Text, out var modelName))
            {
                return;
            }

            DownloadUrlBox.Text = $"{modelName}:{value}";
        }
    }

    public Visibility HuggingFaceOptionsVisibility =>
        string.Equals(SelectedDownloadSource, "Hugging Face", StringComparison.Ordinal)
            ? Visibility.Visible
            : Visibility.Collapsed;

    public Visibility OllamaOptionsVisibility =>
        string.Equals(SelectedDownloadSource, "Ollama Library", StringComparison.Ordinal)
            ? Visibility.Visible
            : Visibility.Collapsed;

    public string ComposerHintText
    {
        get => _composerHintText;
        set => SetProperty(ref _composerHintText, value);
    }

    public bool IsPromptInputEnabled => !_isChatBusy;

    public bool IsSendEnabled => !_isChatBusy;

    public bool IsStopEnabled => _isChatBusy;

    public Visibility EmptyStateVisibility
    {
        get => _emptyStateVisibility;
        set => SetProperty(ref _emptyStateVisibility, value);
    }

    public MainPage()
    {
        InitializeComponent();
        Loaded += MainPage_Loaded;
        Unloaded += MainPage_Unloaded;
        Messages.CollectionChanged += Messages_CollectionChanged;
    }

    public event PropertyChangedEventHandler? PropertyChanged;

    private async void MainPage_Loaded(object sender, RoutedEventArgs e)
    {
        try
        {
            _settings = await SettingsStore.LoadAsync();
            ApplySettingsToUi(_settings);
            ReloadHistoryCollections(_settings);
            ReloadPresetCollection(_settings);
            ServerStatusText = $"Settings loaded from {SettingsStore.GetSettingsFilePath()}";

            Messages.Add(new ChatMessage
            {
                Role = "assistant",
                Content = "Ready. Save or start your llama.cpp server, then send a prompt."
            });
            UpdateDownloadSourceUi();
        }
        catch (Exception ex)
        {
            ServerStatusText = "Failed to load settings: " + ex.Message;
        }
    }

    private async void MainPage_Unloaded(object sender, RoutedEventArgs e)
    {
        try
        {
            _downloadCts?.Cancel();
            _chatCts?.Cancel();
            await _serverService.StopAsync();
        }
        catch
        {
            // Avoid unhandled exceptions on app shutdown.
        }
    }

    private async void OnSaveSettingsClicked(object sender, RoutedEventArgs e)
    {
        if (!TryReadSettingsFromUi(out var settings, out var error))
        {
            ServerStatusText = error;
            return;
        }

        TrackRecentPath(settings.RecentModelPaths, settings.ModelPath);
        TrackRecentPath(settings.RecentServerPaths, settings.LlamaServerPath);

        _settings = settings;
        await SettingsStore.SaveAsync(_settings);
        ReloadHistoryCollections(_settings);
        ReloadPresetCollection(_settings);
        ServerStatusText = "Settings saved.";
    }

    private async void OnStartClicked(object sender, RoutedEventArgs e)
    {
        if (!TryReadSettingsFromUi(out var settings, out var error))
        {
            ServerStatusText = error;
            return;
        }

        TrackRecentPath(settings.RecentModelPaths, settings.ModelPath);
        TrackRecentPath(settings.RecentServerPaths, settings.LlamaServerPath);

        _settings = settings;

        try
        {
            await SettingsStore.SaveAsync(_settings);
            ReloadHistoryCollections(_settings);
            ReloadPresetCollection(_settings);

            ServerStatusText = "Starting llama-server...";
            await _serverService.StartAsync(_settings);

            HeaderStatusText = $"Online ({_settings.Host}:{_settings.Port})";
            StatusDotBrush = new SolidColorBrush(Colors.MediumSeaGreen);
            ServerStatusText = "Server ready.";
        }
        catch (Exception ex)
        {
            HeaderStatusText = "Offline";
            StatusDotBrush = new SolidColorBrush(Colors.IndianRed);
            ServerStatusText = "Start failed: " + ex.Message;
        }
    }

    private async void OnStopClicked(object sender, RoutedEventArgs e)
    {
        await _serverService.StopAsync();
        HeaderStatusText = "Offline";
        StatusDotBrush = new SolidColorBrush(Colors.IndianRed);
        ServerStatusText = "Server stopped.";
    }

    private async void OnSendClicked(object sender, RoutedEventArgs e)
    {
        await SendPromptAsync();
    }

    private async void PromptBox_KeyDown(object sender, KeyRoutedEventArgs e)
    {
        var ctrlDown = Microsoft.UI.Input.InputKeyboardSource
            .GetKeyStateForCurrentThread(Windows.System.VirtualKey.Control)
            .HasFlag(Windows.UI.Core.CoreVirtualKeyStates.Down);
        var shiftDown = Microsoft.UI.Input.InputKeyboardSource
            .GetKeyStateForCurrentThread(Windows.System.VirtualKey.Shift)
            .HasFlag(Windows.UI.Core.CoreVirtualKeyStates.Down);

        if (ctrlDown && e.Key is Windows.System.VirtualKey.Up or Windows.System.VirtualKey.Down)
        {
            e.Handled = true;
            var direction = e.Key == Windows.System.VirtualKey.Up ? -1 : 1;
            NavigatePromptHistory(direction);
            return;
        }

        if (e.Key == Windows.System.VirtualKey.Escape && _isChatBusy)
        {
            e.Handled = true;
            _chatCts?.Cancel();
            return;
        }

        if (e.Key != Windows.System.VirtualKey.Enter || shiftDown)
        {
            return;
        }

        e.Handled = true;
        await SendPromptAsync();
    }

    private async Task SendPromptAsync()
    {
        var prompt = PromptBox.Text?.Trim() ?? string.Empty;
        if (string.IsNullOrWhiteSpace(prompt))
        {
            return;
        }

        if (TryHandleSlashCommand(prompt, out var clearPrompt))
        {
            if (clearPrompt)
            {
                PromptBox.Text = string.Empty;
            }

            ResetPromptHistoryNavigation();
            return;
        }

        if (_isChatBusy)
        {
            ServerStatusText = "Generation in progress. Click Stop to cancel.";
            return;
        }

        if (!_serverService.IsRunning)
        {
            ServerStatusText = "Server is not running. Click Start first.";
            return;
        }

        PromptBox.Text = string.Empty;
        TrackPromptHistory(prompt);
        ResetPromptHistoryNavigation();

        var userMessage = new ChatMessage { Role = "user", Content = prompt };
        Messages.Add(userMessage);

        var assistantMessage = new ChatMessage { Role = "assistant", Content = string.Empty };
        Messages.Add(assistantMessage);
        ScrollToBottom();

        _chatCts?.Dispose();
        _chatCts = new CancellationTokenSource();
        SetChatBusy(true);

        try
        {
            ServerStatusText = "Streaming response...";

            var gotAnyToken = false;
            await foreach (var chunk in _serverService.StreamChatAsync(_settings, Messages.ToList(), _chatCts.Token))
            {
                gotAnyToken = true;
                assistantMessage.Content += chunk;
                ScrollToBottom();
            }

            if (!gotAnyToken)
            {
                assistantMessage.Content = "No response from model.";
            }

            ServerStatusText = "Response received.";
        }
        catch (OperationCanceledException)
        {
            if (string.IsNullOrWhiteSpace(assistantMessage.Content))
            {
                assistantMessage.Content = "Generation canceled.";
            }

            ServerStatusText = "Generation canceled.";
        }
        catch
        {
            try
            {
                ServerStatusText = "Stream failed, retrying once without stream...";
                var fallbackText = await _serverService.ChatAsync(_settings, Messages.ToList(), _chatCts.Token);
                assistantMessage.Content = fallbackText;
                ServerStatusText = "Response received (fallback mode).";
            }
            catch (OperationCanceledException)
            {
                if (string.IsNullOrWhiteSpace(assistantMessage.Content))
                {
                    assistantMessage.Content = "Generation canceled.";
                }

                ServerStatusText = "Generation canceled.";
            }
            catch (Exception ex)
            {
                assistantMessage.Content = "Request failed: " + ex.Message;
                ServerStatusText = "Request failed.";
            }
        }
        finally
        {
            _chatCts?.Dispose();
            _chatCts = null;
            SetChatBusy(false);
        }
    }

    private bool TryReadSettingsFromUi(out AppSettings settings, out string error)
    {
        error = string.Empty;
        settings = new AppSettings();

        if (!int.TryParse(PortBox.Text, out var port) || port <= 0 || port > 65535)
        {
            error = "Port must be a number between 1 and 65535.";
            return false;
        }

        if (!int.TryParse(ContextBox.Text, out var context) || context < 256)
        {
            error = "Context size must be at least 256.";
            return false;
        }

        if (!int.TryParse(ThreadsBox.Text, out var threads) || threads < 1)
        {
            error = "Threads must be at least 1.";
            return false;
        }

        if (!int.TryParse(GpuLayersBox.Text, out var gpuLayers) || gpuLayers < 0)
        {
            error = "GPU layers must be 0 or more.";
            return false;
        }

        if (!float.TryParse(TemperatureBox.Text, out var temperature) || temperature < 0 || temperature > 2)
        {
            error = "Temperature must be between 0 and 2.";
            return false;
        }

        if (!int.TryParse(MaxTokensBox.Text, out var maxTokens) || maxTokens < 1)
        {
            error = "Max tokens must be at least 1.";
            return false;
        }

        settings = new AppSettings
        {
            LlamaServerPath = ServerPathBox.Text.Trim(),
            ModelPath = ModelPathBox.Text.Trim(),
            Host = string.IsNullOrWhiteSpace(HostBox.Text) ? "127.0.0.1" : HostBox.Text.Trim(),
            Port = port,
            ContextSize = context,
            Threads = threads,
            GpuLayers = gpuLayers,
            Temperature = temperature,
            MaxTokens = maxTokens,
            RecentModelPaths = _settings.RecentModelPaths.ToList(),
            RecentServerPaths = _settings.RecentServerPaths.ToList(),
            Presets = _settings.Presets.ToList(),
            DownloadFolder = string.IsNullOrWhiteSpace(DownloadFolderBox.Text)
                ? _settings.DownloadFolder
                : DownloadFolderBox.Text.Trim(),
            RecentModelUrls = _settings.RecentModelUrls.ToList()
        };

        return true;
    }

    private void ApplySettingsToUi(AppSettings settings)
    {
        ServerPathBox.Text = settings.LlamaServerPath;
        ModelPathBox.Text = settings.ModelPath;
        HostBox.Text = settings.Host;
        PortBox.Text = settings.Port.ToString();
        ContextBox.Text = settings.ContextSize.ToString();
        ThreadsBox.Text = settings.Threads.ToString();
        GpuLayersBox.Text = settings.GpuLayers.ToString();
        TemperatureBox.Text = settings.Temperature.ToString("0.0");
        MaxTokensBox.Text = settings.MaxTokens.ToString();
        DownloadFolderBox.Text = settings.DownloadFolder;
    }

    private void ReloadHistoryCollections(AppSettings settings)
    {
        ReplaceCollection(ModelHistory, settings.RecentModelPaths);
        ReplaceCollection(ServerHistory, settings.RecentServerPaths);
        ReplaceCollection(DownloadUrlHistory, settings.RecentModelUrls);
    }

    private void ReloadPresetCollection(AppSettings settings)
    {
        ReplaceCollection(
            PresetNames,
            settings.Presets
                .Where(static p => !string.IsNullOrWhiteSpace(p.Name))
                .Select(static p => p.Name)
                .OrderBy(static n => n, StringComparer.OrdinalIgnoreCase));
    }

    private static void ReplaceCollection(ObservableCollection<string> target, IEnumerable<string> values)
    {
        target.Clear();
        foreach (var value in values.Where(static x => !string.IsNullOrWhiteSpace(x)))
        {
            target.Add(value);
        }
    }

    private static void TrackRecentPath(List<string> history, string value)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            return;
        }

        history.RemoveAll(existing => string.Equals(existing, value, StringComparison.OrdinalIgnoreCase));
        history.Insert(0, value);

        if (history.Count > MaxHistoryEntries)
        {
            history.RemoveRange(MaxHistoryEntries, history.Count - MaxHistoryEntries);
        }
    }

    private async void OnBrowseServerClicked(object sender, RoutedEventArgs e)
    {
        var path = await PickFileAsync(new[] { ".exe" });
        if (!string.IsNullOrWhiteSpace(path))
        {
            ServerPathBox.Text = path;
        }
    }

    private async void OnBrowseModelClicked(object sender, RoutedEventArgs e)
    {
        var path = await PickFileAsync(new[] { ".gguf", ".bin", ".safetensors", ".*" });
        if (!string.IsNullOrWhiteSpace(path))
        {
            ModelPathBox.Text = path;
        }
    }

    private void ModelHistoryBox_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (ModelHistoryBox.SelectedItem is string selected)
        {
            ModelPathBox.Text = selected;
        }
    }

    private void ServerHistoryBox_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (ServerHistoryBox.SelectedItem is string selected)
        {
            ServerPathBox.Text = selected;
        }
    }

    private async void OnSavePresetClicked(object sender, RoutedEventArgs e)
    {
        if (!TryReadSettingsFromUi(out var current, out var error))
        {
            ServerStatusText = error;
            return;
        }

        var presetName = PresetNameBox.Text?.Trim() ?? string.Empty;
        if (string.IsNullOrWhiteSpace(presetName))
        {
            ServerStatusText = "Preset name is required.";
            return;
        }

        var preset = new ModelPreset
        {
            Name = presetName,
            ModelPath = current.ModelPath,
            Host = current.Host,
            Port = current.Port,
            ContextSize = current.ContextSize,
            Threads = current.Threads,
            GpuLayers = current.GpuLayers,
            Temperature = current.Temperature,
            MaxTokens = current.MaxTokens
        };

        _settings.Presets.RemoveAll(p => string.Equals(p.Name, presetName, StringComparison.OrdinalIgnoreCase));
        _settings.Presets.Add(preset);
        await SettingsStore.SaveAsync(_settings);

        ReloadPresetCollection(_settings);
        PresetBox.SelectedItem = preset.Name;
        ServerStatusText = $"Preset '{preset.Name}' saved.";
    }

    private void PresetBox_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (PresetBox.SelectedItem is string selected)
        {
            PresetNameBox.Text = selected;
        }
    }

    private async void OnApplyPresetClicked(object sender, RoutedEventArgs e)
    {
        var selectedName = (PresetBox.SelectedItem as string) ?? PresetNameBox.Text?.Trim();
        if (string.IsNullOrWhiteSpace(selectedName))
        {
            ServerStatusText = "Pick or type a preset name to apply.";
            return;
        }

        var preset = _settings.Presets.FirstOrDefault(p =>
            string.Equals(p.Name, selectedName, StringComparison.OrdinalIgnoreCase));
        if (preset is null)
        {
            ServerStatusText = $"Preset '{selectedName}' not found.";
            return;
        }

        ModelPathBox.Text = preset.ModelPath;
        HostBox.Text = preset.Host;
        PortBox.Text = preset.Port.ToString();
        ContextBox.Text = preset.ContextSize.ToString();
        ThreadsBox.Text = preset.Threads.ToString();
        GpuLayersBox.Text = preset.GpuLayers.ToString();
        TemperatureBox.Text = preset.Temperature.ToString("0.0");
        MaxTokensBox.Text = preset.MaxTokens.ToString();
        PresetNameBox.Text = preset.Name;

        if (TryReadSettingsFromUi(out var settings, out _))
        {
            _settings = settings;
            await SettingsStore.SaveAsync(_settings);
            ReloadHistoryCollections(_settings);
        }

        ServerStatusText = $"Preset '{preset.Name}' applied.";
    }

    private async void OnDeletePresetClicked(object sender, RoutedEventArgs e)
    {
        var selectedName = (PresetBox.SelectedItem as string) ?? PresetNameBox.Text?.Trim();
        if (string.IsNullOrWhiteSpace(selectedName))
        {
            ServerStatusText = "Pick a preset to delete.";
            return;
        }

        var removed = _settings.Presets.RemoveAll(p =>
            string.Equals(p.Name, selectedName, StringComparison.OrdinalIgnoreCase));
        if (removed == 0)
        {
            ServerStatusText = $"Preset '{selectedName}' not found.";
            return;
        }

        await SettingsStore.SaveAsync(_settings);
        ReloadPresetCollection(_settings);
        PresetBox.SelectedItem = null;
        PresetNameBox.Text = string.Empty;
        ServerStatusText = $"Preset '{selectedName}' deleted.";
    }

    private void DownloadUrlHistoryBox_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (DownloadUrlHistoryBox.SelectedItem is string selected)
        {
            if (TryParseDownloadHistoryEntry(selected, out var source, out var reference))
            {
                SelectedDownloadSource = source;
                DownloadUrlBox.Text = reference;
                return;
            }

            DownloadUrlBox.Text = selected;
        }
    }

    private async void OnBrowseDownloadFolderClicked(object sender, RoutedEventArgs e)
    {
        var path = await PickFolderAsync();
        if (!string.IsNullOrWhiteSpace(path))
        {
            DownloadFolderBox.Text = path;
        }
    }

    private async void OnDownloadModelClicked(object sender, RoutedEventArgs e)
    {
        if (_downloadCts is not null)
        {
            DownloadProgressText = "A download is already running.";
            return;
        }

        var source = SelectedDownloadSource;
        var reference = DownloadUrlBox.Text?.Trim() ?? string.Empty;

        if (!TryReadSettingsFromUi(out var settings, out var error))
        {
            ServerStatusText = error;
            return;
        }

        var folder = settings.DownloadFolder;
        if (string.IsNullOrWhiteSpace(folder))
        {
            DownloadProgressText = "Choose a download folder.";
            return;
        }

        Directory.CreateDirectory(folder);

        _downloadCts = new CancellationTokenSource();
        DownloadProgressBar.Value = 0;
        DownloadProgressText = "Resolving model reference...";
        string? outputPath = null;

        var progress = new Progress<(long downloadedBytes, long? totalBytes)>(state =>
        {
            var (downloaded, total) = state;
            if (total is > 0)
            {
                var percent = downloaded * 100d / total.Value;
                DownloadProgressBar.Value = Math.Clamp(percent, 0d, 100d);
                DownloadProgressText = $"{percent:0.0}% ({FormatBytes(downloaded)} / {FormatBytes(total.Value)})";
            }
            else
            {
                DownloadProgressText = $"{FormatBytes(downloaded)} downloaded";
            }
        });

        try
        {
            var resolved = await _modelReferenceResolver.ResolveAsync(
                source,
                reference,
                HfTokenBox.Password,
                _downloadCts.Token);

            var fileName = DownloadFileNameBox.Text?.Trim();
            if (string.IsNullOrWhiteSpace(fileName))
            {
                fileName = resolved.SuggestedFileName;
            }

            if (string.IsNullOrWhiteSpace(fileName))
            {
                fileName = "model.gguf";
            }

            outputPath = Path.Combine(folder, fileName);
            DownloadProgressText = "Starting download...";

            await _downloadService.DownloadAsync(
                resolved.DownloadUrl,
                outputPath,
                progress,
                resolved.RequestHeaders,
                _downloadCts.Token);

            ModelPathBox.Text = outputPath;
            settings.ModelPath = outputPath;
            TrackRecentPath(settings.RecentModelPaths, outputPath);
            TrackRecentPath(settings.RecentModelUrls, ComposeDownloadHistoryEntry(source, reference));

            _settings = settings;
            await SettingsStore.SaveAsync(_settings);
            ReloadHistoryCollections(_settings);

            DownloadProgressBar.Value = 100;
            DownloadProgressText = $"Download complete: {outputPath}";
            ServerStatusText = "Model downloaded and selected.";
        }
        catch (OperationCanceledException)
        {
            DownloadProgressText = "Download canceled.";
            if (!string.IsNullOrWhiteSpace(outputPath) && File.Exists(outputPath))
            {
                try { File.Delete(outputPath); } catch { }
            }
        }
        catch (Exception ex)
        {
            DownloadProgressText = "Download failed: " + ex.Message;
        }
        finally
        {
            _downloadCts.Dispose();
            _downloadCts = null;
        }
    }

    private async void OnPreviewHuggingFaceFilesClicked(object sender, RoutedEventArgs e)
    {
        if (!string.Equals(SelectedDownloadSource, "Hugging Face", StringComparison.Ordinal))
        {
            HuggingFaceFilesStatusText = "Switch source to Hugging Face first.";
            return;
        }

        if (!TryGetHuggingFaceRepoId(DownloadUrlBox.Text, out var repoId))
        {
            HuggingFaceFilesStatusText = "Enter Hugging Face repo as owner/repo first.";
            return;
        }

        HuggingFaceFilesStatusText = "Loading files...";
        try
        {
            var files = await _modelReferenceResolver.ListHuggingFaceGgufFilesAsync(
                repoId,
                HfTokenBox.Password,
                CancellationToken.None);

            HuggingFaceGgufFiles.Clear();
            foreach (var file in files)
            {
                HuggingFaceGgufFiles.Add(file);
            }

            if (files.Count == 0)
            {
                HuggingFaceFilesStatusText = "No .gguf files found in this repo.";
                SelectedHuggingFaceGgufFile = null;
            }
            else
            {
                HuggingFaceFilesStatusText = $"{files.Count} .gguf file(s) loaded.";
                SelectedHuggingFaceGgufFile = files[0];
            }
        }
        catch (Exception ex)
        {
            HuggingFaceFilesStatusText = "Failed to load files: " + ex.Message;
        }
    }

    private async void OnPreviewOllamaTagsClicked(object sender, RoutedEventArgs e)
    {
        if (!string.Equals(SelectedDownloadSource, "Ollama Library", StringComparison.Ordinal))
        {
            OllamaTagsStatusText = "Switch source to Ollama Library first.";
            return;
        }

        if (!TryGetOllamaModelName(DownloadUrlBox.Text, out var modelName))
        {
            OllamaTagsStatusText = "Enter model name first, for example 'llama3' or 'llama3:8b'.";
            return;
        }

        OllamaTagsStatusText = "Loading tags...";
        try
        {
            var tags = await _modelReferenceResolver.ListOllamaTagsAsync(
                modelName,
                CancellationToken.None);

            OllamaTags.Clear();
            foreach (var tag in tags)
            {
                OllamaTags.Add(tag);
            }

            if (tags.Count == 0)
            {
                OllamaTagsStatusText = "No tags found for this model.";
                SelectedOllamaTag = null;
            }
            else
            {
                OllamaTagsStatusText = $"{tags.Count} tag(s) loaded.";
                SelectedOllamaTag = tags[0];
            }
        }
        catch (Exception ex)
        {
            OllamaTagsStatusText = "Failed to load tags: " + ex.Message;
        }
    }

    private void OnCancelDownloadClicked(object sender, RoutedEventArgs e)
    {
        _downloadCts?.Cancel();
    }

    private void OnStopGenerationClicked(object sender, RoutedEventArgs e)
    {
        _chatCts?.Cancel();
    }

    private void OnCopyMessageClicked(object sender, RoutedEventArgs e)
    {
        if (sender is not FrameworkElement element || element.Tag is not ChatMessage message)
        {
            return;
        }

        if (string.IsNullOrWhiteSpace(message.Content))
        {
            return;
        }

        var data = new DataPackage();
        data.SetText(message.Content);
        Clipboard.SetContent(data);
        ServerStatusText = "Message copied to clipboard.";
    }

    private void OnUseAsPromptClicked(object sender, RoutedEventArgs e)
    {
        if (sender is not FrameworkElement element || element.Tag is not ChatMessage message)
        {
            return;
        }

        PromptBox.Text = message.Content;
        PromptBox.Focus(FocusState.Programmatic);
        ServerStatusText = "Message copied into prompt editor.";
    }

    private void OnClearChatClicked(object sender, RoutedEventArgs e)
    {
        ClearChatConversation();
    }

    private void OnCopyLastReplyClicked(object sender, RoutedEventArgs e)
    {
        _ = CopyLastAssistantReplyToClipboard();
    }

    private void OnStarterPromptClicked(object sender, RoutedEventArgs e)
    {
        if (sender is not Button button)
        {
            return;
        }

        PromptBox.Text = button.Content?.ToString() ?? string.Empty;
        PromptBox.Focus(FocusState.Programmatic);
    }

    private static async Task<string?> PickFileAsync(IEnumerable<string> extensions)
    {
        if (App.MainAppWindow is null)
        {
            return null;
        }

        var picker = new FileOpenPicker
        {
            SuggestedStartLocation = PickerLocationId.ComputerFolder,
            ViewMode = PickerViewMode.List
        };

        foreach (var extension in extensions)
        {
            picker.FileTypeFilter.Add(extension);
        }

        var windowHandle = WindowNative.GetWindowHandle(App.MainAppWindow);
        InitializeWithWindow.Initialize(picker, windowHandle);

        var file = await picker.PickSingleFileAsync();
        return file?.Path;
    }

    private static async Task<string?> PickFolderAsync()
    {
        if (App.MainAppWindow is null)
        {
            return null;
        }

        var picker = new FolderPicker
        {
            SuggestedStartLocation = PickerLocationId.ComputerFolder,
            ViewMode = PickerViewMode.List
        };

        picker.FileTypeFilter.Add("*");

        var windowHandle = WindowNative.GetWindowHandle(App.MainAppWindow);
        InitializeWithWindow.Initialize(picker, windowHandle);

        var folder = await picker.PickSingleFolderAsync();
        return folder?.Path;
    }

    private static string FormatBytes(long bytes)
    {
        string[] units = ["B", "KB", "MB", "GB", "TB"];
        double size = bytes;
        var unitIndex = 0;
        while (size >= 1024 && unitIndex < units.Length - 1)
        {
            size /= 1024;
            unitIndex++;
        }

        return $"{size:0.##} {units[unitIndex]}";
    }

    private void UpdateDownloadSourceUi()
    {
        switch (SelectedDownloadSource)
        {
            case "Hugging Face":
                DownloadInputLabel = "Hugging Face model (owner/repo or owner/repo::file.gguf)";
                DownloadInputPlaceholder = "TheBloke/Llama-2-7B-GGUF::llama-2-7b.Q4_K_M.gguf";
                DownloadSourceHintText = "If file is omitted, the app auto-picks a .gguf from the repo.";
                HuggingFaceFilesStatusText = "Load .gguf file list from repo.";
                OllamaTagsStatusText = "Ollama tags are disabled.";
                OllamaTags.Clear();
                SelectedOllamaTag = null;
                break;
            case "Ollama Library":
                DownloadInputLabel = "Ollama model (model[:tag])";
                DownloadInputPlaceholder = "llama3:8b";
                DownloadSourceHintText = "Also supports full names like 'library/phi3:latest'.";
                HuggingFaceFilesStatusText = "Hugging Face file list is disabled.";
                HuggingFaceGgufFiles.Clear();
                SelectedHuggingFaceGgufFile = null;
                OllamaTagsStatusText = "Load tags from Ollama registry.";
                break;
            default:
                DownloadInputLabel = "Model URL (.gguf direct link)";
                DownloadInputPlaceholder = "https://example.com/model.gguf";
                DownloadSourceHintText = "Paste a direct link to a .gguf model file.";
                HuggingFaceFilesStatusText = "Hugging Face file list is disabled.";
                HuggingFaceGgufFiles.Clear();
                SelectedHuggingFaceGgufFile = null;
                OllamaTagsStatusText = "Ollama tags are disabled.";
                OllamaTags.Clear();
                SelectedOllamaTag = null;
                break;
        }

        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(HuggingFaceOptionsVisibility)));
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(OllamaOptionsVisibility)));
    }

    private static string ComposeDownloadHistoryEntry(string source, string reference)
        => $"[{source}] {reference}";

    private static bool TryParseDownloadHistoryEntry(string entry, out string source, out string reference)
    {
        source = string.Empty;
        reference = entry;

        if (!entry.StartsWith("[", StringComparison.Ordinal))
        {
            return false;
        }

        var close = entry.IndexOf("] ", StringComparison.Ordinal);
        if (close < 0)
        {
            return false;
        }

        source = entry[1..close];
        reference = entry[(close + 2)..];
        return !string.IsNullOrWhiteSpace(source) && !string.IsNullOrWhiteSpace(reference);
    }

    private static bool TryGetHuggingFaceRepoId(string? value, out string repoId)
    {
        repoId = string.Empty;
        var text = value?.Trim() ?? string.Empty;
        if (string.IsNullOrWhiteSpace(text))
        {
            return false;
        }

        var parts = text.Split("::", 2, StringSplitOptions.TrimEntries);
        var candidate = parts[0];
        if (candidate.Contains('/', StringComparison.Ordinal) &&
            !candidate.StartsWith("http://", StringComparison.OrdinalIgnoreCase) &&
            !candidate.StartsWith("https://", StringComparison.OrdinalIgnoreCase))
        {
            repoId = candidate;
            return true;
        }

        if (!Uri.TryCreate(candidate, UriKind.Absolute, out var uri) ||
            !uri.Host.Contains("huggingface.co", StringComparison.OrdinalIgnoreCase))
        {
            return false;
        }

        var segments = uri.AbsolutePath
            .Split('/', StringSplitOptions.RemoveEmptyEntries);
        if (segments.Length < 2)
        {
            return false;
        }

        if (string.Equals(segments[0], "models", StringComparison.OrdinalIgnoreCase) && segments.Length >= 3)
        {
            repoId = $"{segments[1]}/{segments[2]}";
            return true;
        }

        repoId = $"{segments[0]}/{segments[1]}";
        return true;
    }

    private static bool TryGetOllamaModelName(string? value, out string modelName)
    {
        modelName = string.Empty;
        var text = value?.Trim() ?? string.Empty;
        if (string.IsNullOrWhiteSpace(text))
        {
            return false;
        }

        if (text.StartsWith("https://ollama.com/library/", StringComparison.OrdinalIgnoreCase))
        {
            text = text["https://ollama.com/library/".Length..];
        }
        else if (text.StartsWith("ollama.com/library/", StringComparison.OrdinalIgnoreCase))
        {
            text = text["ollama.com/library/".Length..];
        }

        if (text.StartsWith("library/", StringComparison.OrdinalIgnoreCase))
        {
            text = text["library/".Length..];
        }

        var colon = text.LastIndexOf(':');
        modelName = colon > 0 ? text[..colon] : text;
        return !string.IsNullOrWhiteSpace(modelName);
    }

    private void ScrollToBottom()
    {
        if (Messages.Count > 0)
        {
            ChatList.ScrollIntoView(Messages[^1]);
        }
    }

    private void Messages_CollectionChanged(object? sender, NotifyCollectionChangedEventArgs e)
    {
        var hasUserMessages = Messages.Any(static m => string.Equals(m.Role, "user", StringComparison.OrdinalIgnoreCase));
        EmptyStateVisibility = hasUserMessages ? Visibility.Collapsed : Visibility.Visible;
    }

    private void SetChatBusy(bool isBusy)
    {
        if (_isChatBusy == isBusy)
        {
            return;
        }

        _isChatBusy = isBusy;
        ComposerHintText = _isChatBusy
            ? "Generating... click Stop or press Esc to cancel."
            : "Enter send, Shift+Enter new line, Ctrl+Up/Down history, /help commands.";

        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(IsPromptInputEnabled)));
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(IsSendEnabled)));
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(IsStopEnabled)));
    }

    private void TrackPromptHistory(string prompt)
    {
        _promptHistory.RemoveAll(existing => string.Equals(existing, prompt, StringComparison.Ordinal));
        _promptHistory.Add(prompt);
        if (_promptHistory.Count > MaxPromptHistoryEntries)
        {
            _promptHistory.RemoveAt(0);
        }
    }

    private void NavigatePromptHistory(int direction)
    {
        if (_promptHistory.Count == 0)
        {
            return;
        }

        if (_promptHistoryIndex == -1)
        {
            if (direction > 0)
            {
                return;
            }

            _promptDraftBeforeHistory = PromptBox.Text;
            _promptHistoryIndex = _promptHistory.Count - 1;
        }
        else
        {
            _promptHistoryIndex += direction;
            if (_promptHistoryIndex < 0)
            {
                _promptHistoryIndex = 0;
            }

            if (_promptHistoryIndex >= _promptHistory.Count)
            {
                PromptBox.Text = _promptDraftBeforeHistory;
                PromptBox.SelectionStart = PromptBox.Text.Length;
                _promptHistoryIndex = -1;
                return;
            }
        }

        PromptBox.Text = _promptHistory[_promptHistoryIndex];
        PromptBox.SelectionStart = PromptBox.Text.Length;
    }

    private void ResetPromptHistoryNavigation()
    {
        _promptHistoryIndex = -1;
        _promptDraftBeforeHistory = string.Empty;
    }

    private bool TryHandleSlashCommand(string commandText, out bool clearPrompt)
    {
        clearPrompt = true;

        var command = commandText.Split(' ', 2, StringSplitOptions.RemoveEmptyEntries)[0];
        if (!command.StartsWith("/", StringComparison.Ordinal))
        {
            return false;
        }

        switch (command.ToLowerInvariant())
        {
            case "/help":
                ServerStatusText = "Commands: /help, /clear, /copylast, /stop, /status, /reuselast";
                return true;
            case "/clear":
                ClearChatConversation();
                return true;
            case "/copylast":
                _ = CopyLastAssistantReplyToClipboard();
                return true;
            case "/stop":
                if (_isChatBusy)
                {
                    _chatCts?.Cancel();
                    ServerStatusText = "Stopping generation...";
                }
                else
                {
                    ServerStatusText = "No active generation to stop.";
                }

                return true;
            case "/status":
                ServerStatusText = _serverService.IsRunning
                    ? $"Server running on {_settings.Host}:{_settings.Port}."
                    : "Server is stopped.";
                return true;
            case "/reuselast":
                var lastAssistant = GetLastAssistantMessage();
                if (lastAssistant is null)
                {
                    ServerStatusText = "No assistant message available.";
                    return true;
                }

                PromptBox.Text = lastAssistant.Content;
                PromptBox.Focus(FocusState.Programmatic);
                ServerStatusText = "Last assistant message moved to prompt.";
                clearPrompt = false;
                return true;
            default:
                ServerStatusText = $"Unknown command '{command}'. Type /help.";
                return true;
        }
    }

    private ChatMessage? GetLastAssistantMessage()
        => Messages.LastOrDefault(static m =>
            string.Equals(m.Role, "assistant", StringComparison.OrdinalIgnoreCase) &&
            !string.IsNullOrWhiteSpace(m.Content));

    private bool CopyLastAssistantReplyToClipboard()
    {
        var lastAssistant = GetLastAssistantMessage();
        if (lastAssistant is null)
        {
            ServerStatusText = "No assistant reply available to copy.";
            return false;
        }

        var data = new DataPackage();
        data.SetText(lastAssistant.Content);
        Clipboard.SetContent(data);
        ServerStatusText = "Last assistant reply copied.";
        return true;
    }

    private void ClearChatConversation()
    {
        Messages.Clear();
        Messages.Add(new ChatMessage
        {
            Role = "assistant",
            Content = "Chat cleared. Ask a new prompt when ready."
        });
        ServerStatusText = "Conversation cleared.";
    }

    private bool SetProperty<T>(ref T backingField, T value, [CallerMemberName] string? propertyName = null)
    {
        if (EqualityComparer<T>.Default.Equals(backingField, value))
        {
            return false;
        }

        backingField = value;
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
        return true;
    }
}
