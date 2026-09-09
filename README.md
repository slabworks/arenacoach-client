# Arena Coach desktop companion

A Tauri + React companion that reads MTG Arena’s game log and syncs completed matches to Arena Coach.

Run `bun install`, then `bun run tauri dev` to launch the desktop app. `bun run dev` starts only the frontend; game reading requires the native app.

Open the gear button for saved preferences:

- **Developer mode** defaults to off and selects `https://arenacoach-web.test` when enabled, or `https://arenacoach.com` when disabled. Changing environments signs you out. The endpoint no longer depends on the build type, a manual URL, or an environment override. Existing settings from before this change require signing in again.
- **Show debug info** defaults to off and reveals file paths, watcher state, and connection diagnostics.
- **Replay fixture match** is available in settings only with developer mode enabled. Native replay commands also require developer mode.

The main status automatically reflects whether Arena’s game file was found. If needed, enable **Options → Account → Detailed Logs (Plugin Support)** in Arena and restart it.

Validation: `bun run build` and `cargo test --manifest-path src-tauri/Cargo.toml`.
