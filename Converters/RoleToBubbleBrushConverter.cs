using Microsoft.UI.Xaml.Data;
using Microsoft.UI.Xaml.Media;

namespace LlamaCppDesk.Converters;

public sealed class RoleToBubbleBrushConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        var role = value as string;
        var key = string.Equals(role, "user", StringComparison.OrdinalIgnoreCase)
            ? "UserBubbleBrush"
            : "AssistantBubbleBrush";

        return Application.Current.Resources[key] as Brush ??
               Application.Current.Resources["SystemControlBackgroundChromeMediumLowBrush"] as Brush ??
               new SolidColorBrush(Windows.UI.Color.FromArgb(255, 48, 52, 60));
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language)
        => throw new NotSupportedException();
}