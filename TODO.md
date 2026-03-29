# TODO

## P0

- Fix Ollama model tag lookup for library models like `llama3`.
  - Current error: `Ollama tags request failed with HTTP status 404 Not Found`
  - Verify the correct Ollama tags endpoint and repo path normalization.
  - Confirm the UI can resolve tags for common models without requiring a manual URL.

## P1

- Add a true remote-editing mode for the app.
  - Run `llama-server` and the Tauri backend on one machine, then manage settings from another machine on the same LAN.
  - Define a browser-accessible backend API for settings, presets, history, downloads, and server lifecycle.
  - Add authentication for remote access before exposing any control endpoints on the network.

- Make browser mode explicit.
  - Show when the app is running in desktop mode vs browser-only mode.
  - Disable or replace Tauri-only actions when the UI is opened in a browser.
  - Provide a clear explanation of what features require the desktop runtime.

## P2

- Add graceful local storage or sync for remote settings changes.
- Review download limits and cancellation behavior for large model files.
- Add a network-safe update path for models and presets.
- Improve error messages so backend failures point to the failing command or endpoint.

## Notes

- Browser mode is currently frontend-only.
- Desktop mode is the full app path through Tauri.
- The long-term goal is to support editing the same app from another PC on the same network.
