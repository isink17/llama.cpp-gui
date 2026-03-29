# UI scaffold

Minimal Vite + React + TypeScript frontend scaffold for the Tauri migration.

## Run

```bash
npm install
npm run dev
```

`npm run dev` starts the app on port `1420` using Vite's native config loader.

For the Tauri desktop shell, use the root `cargo run --manifest-path src-tauri/Cargo.toml` command. Tauri uses the same `npm --prefix ../ui run dev` startup path and loads the app from `http://localhost:1420`.

## Build

```bash
npm run build
```

## Preview

```bash
npm run preview
```
