using Microsoft.UI.Xaml.Navigation;

namespace LlamaCppDesk
{
    public partial class App : Application
    {
        public static Window? MainAppWindow { get; private set; }

        public App()
        {
            this.InitializeComponent();
        }

        protected override void OnLaunched(LaunchActivatedEventArgs e)
        {
            MainAppWindow ??= new Window
            {
                Title = "Llama.cpp Desk"
            };

            if (MainAppWindow.Content is not Frame rootFrame)
            {
                rootFrame = new Frame();
                rootFrame.NavigationFailed += OnNavigationFailed;
                MainAppWindow.Content = rootFrame;
            }

            _ = rootFrame.Navigate(typeof(MainPage), e.Arguments);
            MainAppWindow.Activate();
        }

        void OnNavigationFailed(object sender, NavigationFailedEventArgs e)
        {
            throw new Exception("Failed to load Page " + e.SourcePageType.FullName);
        }
    }
}
