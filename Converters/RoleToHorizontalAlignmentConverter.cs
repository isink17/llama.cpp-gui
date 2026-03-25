using Microsoft.UI.Xaml.Data;

namespace LlamaCppDesk.Converters;

public sealed class RoleToHorizontalAlignmentConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        var role = value as string;
        return string.Equals(role, "user", StringComparison.OrdinalIgnoreCase)
            ? HorizontalAlignment.Right
            : HorizontalAlignment.Left;
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language)
        => throw new NotSupportedException();
}