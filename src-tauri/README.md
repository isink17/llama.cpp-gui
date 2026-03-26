# src-tauri

Minimal Tauri backend scaffold for the migration baseline.

## Run

From the repository root (first install UI dependencies):

```powershell
npm --prefix ui install
cargo run --manifest-path src-tauri/Cargo.toml
```

## Notes

- Tauri dev mode starts the UI server with:
  - `npm --prefix ../ui run dev -- --port 1420 --strictPort`
- Tauri build mode uses:
  - `npm --prefix ../ui run build`
- The only current command is `ping`, which returns `pong` and is meant for early wiring checks.
