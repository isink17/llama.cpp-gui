# Remote Mode

This project now supports two runtime paths:

- `desktop`: the normal Tauri shell with local IPC
- `browser`: a frontend client that talks to a remote Tauri backend over HTTP

## Desktop Mode

Run the app through the Tauri shell. All settings, downloads, chat, and process actions go through local IPC.

## Browser Mode

Browser mode requires a remote backend started on the machine that runs `llama-server`.

### Required Environment

- `LLAMACPPDESK_REMOTE_BIND`
  - Example: `127.0.0.1:8080`
  - Use a loopback bind for local-only access.
  - Use a LAN bind only when you intend to expose the backend to trusted clients.
- `LLAMACPPDESK_REMOTE_TOKEN`
  - Bearer token required by every HTTP request.

### Browser Setup

1. Open the frontend in the browser.
2. Enter the remote base URL.
3. Enter the bearer token.
4. Save the remote config.

After that, the browser shell can control:

- settings
- presets
- history
- `llama-server` process lifecycle
- downloads
- chat streaming

## Security Notes

- Keep the token private.
- Prefer loopback binding unless remote LAN access is truly needed.
- If you bind to a non-loopback address, the backend prints a warning on startup.

## Remaining Gaps

- The browser path still uses explicit destination paths for downloads.
- File and folder pickers remain desktop-only.
- Remote sync for local file paths is intentionally limited to the current configured backend state.
