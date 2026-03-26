# Smoke Test Baseline

This folder contains lightweight smoke validation for migration increments.

## Goal

Provide fast confidence checks that the current migration slice is still runnable and does not regress critical flows.

## Usage

- Use `smoke-checklist.md` as a manual validation gate per migration PR.
- Add automated smoke scripts in this folder as Tauri app scaffolding becomes available.

## Scope

- Startup checks
- Settings/persistence checks
- Chat request/streaming checks
- Downloader checks
- Basic packaging checks
