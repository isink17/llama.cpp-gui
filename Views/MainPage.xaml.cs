using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Runtime.CompilerServices;
using LlamaCppDesk.Models;
using LlamaCppDesk.Services;
using Microsoft.UI;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Windows.Storage.Pickers;
using WinRT.Interop;

namespace LlamaCppDesk.Views;

public sealed partial class MainPage : Page, INotifyPropertyChanged
{
    private const int MaxHistoryEntries = 12;

    private readonly LlamaServerService _serverService = new();
    private readonly ModelDownloadService _downloadService = new();
    private AppSettings _settings = new();
    private CancellationTokenSource? _downloadCts;

    private string _serverStatusText = "Server is stopped.";
    private string _headerStatusText = "Offline";
    private string _downloadProgressText = "Idle.";
    private Brush _statusDotBrush = new SolidColorBrush(Colors.IndianRed);

    public ObservableCollection<ChatMessage> Messages { get; } = new();

    public ObservableCollection<string> ModelHistory { get; } = new();

    public ObservableCollection<string> ServerHistory { get; } = new();

    public ObservableCollection<string> PresetNames { get; } = new();

    public ObservableCollection<string> DownloadUrlHistory { get; } = new();

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

    public MainPage()
    {
        InitializeComponent();
        Loaded += MainPage_Loaded;
        Unloaded += MainPage_Unloaded;
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
        if (e.Key != Windows.System.VirtualKey.Enter)
        {
            return;
        }

        var shiftDown = Microsoft.UI.Input.InputKeyboardSource
            .GetKeyStateForCurrentThread(Windows.System.VirtualKey.Shift)
            .HasFlag(Windows.UI.Core.CoreVirtualKeyStates.Down);

        if (shiftDown)
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

        if (!_serverService.IsRunning)
        {
            ServerStatusText = "Server is not running. Click Start first.";
            return;
        }

        PromptBox.Text = string.Empty;

        var userMessage = new ChatMessage { Role = "user", Content = prompt };
        Messages.Add(userMessage);

        var assistantMessage = new ChatMessage { Role = "assistant", Content = string.Empty };
        Messages.Add(assistantMessage);
        ScrollToBottom();

        try
        {
            ServerStatusText = "Streaming response...";

            var gotAnyToken = false;
            await foreach (var chunk in _serverService.StreamChatAsync(_settings, Messages.ToList()))
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
        catch
        {
            try
            {
                ServerStatusText = "Stream failed, retrying once without stream...";
                var fallbackText = await _serverService.ChatAsync(_settings, Messages.ToList());
                assistantMessage.Content = fallbackText;
                ServerStatusText = "Response received (fallback mode).";
            }
            catch (Exception ex)
            {
                assistantMessage.Content = "Request failed: " + ex.Message;
                ServerStatusText = "Request failed.";
            }
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

        var url = DownloadUrlBox.Text?.Trim() ?? string.Empty;
        if (!Uri.TryCreate(url, UriKind.Absolute, out var uri))
        {
            DownloadProgressText = "Enter a valid model URL.";
            return;
        }

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

        var fileName = DownloadFileNameBox.Text?.Trim();
        if (string.IsNullOrWhiteSpace(fileName))
        {
            fileName = Path.GetFileName(uri.LocalPath);
        }

        if (string.IsNullOrWhiteSpace(fileName))
        {
            fileName = "model.gguf";
        }

        var outputPath = Path.Combine(folder, fileName);
        _downloadCts = new CancellationTokenSource();
        DownloadProgressBar.Value = 0;
        DownloadProgressText = "Starting download...";

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
            await _downloadService.DownloadAsync(url, outputPath, progress, _downloadCts.Token);

            ModelPathBox.Text = outputPath;
            settings.ModelPath = outputPath;
            TrackRecentPath(settings.RecentModelPaths, outputPath);
            TrackRecentPath(settings.RecentModelUrls, url);

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
            if (File.Exists(outputPath))
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

    private void OnCancelDownloadClicked(object sender, RoutedEventArgs e)
    {
        _downloadCts?.Cancel();
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

    private void ScrollToBottom()
    {
        if (Messages.Count > 0)
        {
            ChatList.ScrollIntoView(Messages[^1]);
        }
    }

    private void SetProperty<T>(ref T backingField, T value, [CallerMemberName] string? propertyName = null)
    {
        if (EqualityComparer<T>.Default.Equals(backingField, value))
        {
            return;
        }

        backingField = value;
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
    }
}
