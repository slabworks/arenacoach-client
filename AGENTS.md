# Agents

Desktop companion for Arena Coach. Tauri 2, React, TypeScript, Bun, and Rust.

Read [CONTRIBUTING.md](CONTRIBUTING.md) and [README.md](README.md) before editing.

## Commands

- `bun test` — frontend tests
- `bun run build` — production frontend build
- `cargo test --manifest-path src-tauri/Cargo.toml` — Rust tests
- `bun run desktop` — the real app (needed to read Arena)
- `bun run dev` — window UI only

## Layout

- `src/` — React UI
- `src-tauri/src/` — log reading, match assembly, and uploads
- `test/` — Bun tests and fixtures

## Platform

macOS and Windows. Arena’s log is `Player.log` in the usual MTGA folder for
that OS (`Library/Logs/...` on Mac, `AppData\LocalLow\...` on Windows). Linux
is not supported for log reading. Commands below work in PowerShell as well as
a Unix shell. Windows builds need the MSVC C++ tools and WebView2; see
[CONTRIBUTING.md](CONTRIBUTING.md).

## Constraints

- Post-game only. No live overlay or solver.
- Upload a finished match, not the raw Arena log.
- Leave names and account ids off the upload.
- Follow existing code style. Do not add comments that restate the code.

Agentic PRs are welcome. Tests must pass.
