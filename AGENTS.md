# Agents

Desktop companion for Arena Coach. Tauri 2, React, TypeScript, Bun, and Rust.

Read [CONTRIBUTING.md](CONTRIBUTING.md) and [README.md](README.md) before editing.

## Commands

- `bun test` — frontend tests
- `bun run build` — TypeScript + Vite production build
- `cargo test --manifest-path src-tauri/Cargo.toml` — Rust tests
- `bun run desktop` — native app (required to read the Arena log)
- `bun run dev` — frontend only

## Layout

- `src/` — React UI
- `src-tauri/src/` — watcher, log follower, match assembler, HTTP poster
- `test/` — Bun tests and fixtures

## Constraints

- Post-game only. No live overlay or solver.
- Parsed match payloads only. No raw `Player.log` uploads.
- Strip identity on device.
- No model keys in this repo or binary.
- Follow existing code style. Do not add comments that restate the code.

Agentic PRs are welcome. Tests must pass.
