using System.ComponentModel;
using System.Runtime.CompilerServices;

namespace LlamaCppDesk.Models;

public sealed class ChatMessage : INotifyPropertyChanged
{
    private string _role = string.Empty;
    private string _content = string.Empty;

    public string Role
    {
        get => _role;
        set => SetProperty(ref _role, value);
    }

    public string Content
    {
        get => _content;
        set => SetProperty(ref _content, value);
    }

    public bool IsUser => Role.Equals("user", StringComparison.OrdinalIgnoreCase);

    public event PropertyChangedEventHandler? PropertyChanged;

    private void SetProperty<T>(ref T backingField, T value, [CallerMemberName] string? propertyName = null)
    {
        if (EqualityComparer<T>.Default.Equals(backingField, value))
        {
            return;
        }

        backingField = value;
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));

        if (propertyName is nameof(Role))
        {
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(IsUser)));
        }
    }
}
