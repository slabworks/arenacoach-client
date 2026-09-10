# Arena Coach desktop companion

Unofficial MTG Arena post-game companion. Not affiliated with Wizards of the Coast.

This Tauri app tails Arena’s local game log, assembles a finished match, and
syncs it to [arenacoach-web](https://github.com/slabworks/arenacoach-web). The
watcher never sees the model key. Identity fields are stripped on the device.

## Table of Contents

- [Features](#features)
- [Tech Stack](#tech-stack)
- [Getting Started](#getting-started)
- [Development](#development)
- [Testing and Quality](#testing-and-quality)
- [Contributing](#contributing)
- [Support](#support)
- [License](#license)

## Features

- Reads the player’s own opted-in Arena log (`Player.log`)
- Assembles one finished match into a compact JSON payload
- Posts from Rust so local HTTPS / CORS is not a browser problem
- Sign-in against the Arena Coach website
- Status for watching, match in progress, upload, and errors
- Developer mode for a local web host vs production

Enable **Options → Account → Detailed Logs (Plugin Support)** in Arena and
restart the client if the log file is missing.

## Tech Stack

- Tauri 2
- React 19 and TypeScript
- Bun
- Rust (reqwest, tokio)

Mac is the first target. Windows log-path support can exist without a shipped installer.

## Getting Started

### Prerequisites

- [Bun](https://bun.sh)
- A recent stable [Rust](https://rustup.rs) toolchain
- Tauri 2 system libraries for your OS ([Tauri prerequisites](https://v2.tauri.app/start/prerequisites/))

### Installation

```bash
git clone https://github.com/slabworks/arenacoach-client.git
cd arenacoach-client
bun install
bun run desktop
```

`bun run dev` starts only the frontend. Game reading requires the native app.

Open the gear button for saved preferences:

- **Developer mode** defaults to off and selects `https://arenacoach-web.test` when enabled, or `https://arenacoach.com` when disabled. Changing environments signs you out.
- **Show debug info** defaults to off and reveals file paths, watcher state, and connection diagnostics.
- **Replay fixture match** is available in settings only with developer mode enabled.

## Development

```bash
bun run desktop
```

Default Mac log path:

`~/Library/Logs/Wizards Of The Coast/MTGA/Player.log`

## Testing and Quality

```bash
bun test
bun run build
cargo test --manifest-path src-tauri/Cargo.toml
```

## Contributing

Contributions are welcome through pull requests. Agentic code is welcome
(Claude, Cursor, Codex, and others). See [CONTRIBUTING.md](CONTRIBUTING.md).

1. Fork the repository.
2. Create a feature branch from `main`.
3. Make your changes with clear commit messages.
4. Add or update tests for any behavior change.
5. Run the validation commands above.
6. Open a pull request.

Please follow the [Code of Conduct](CODE_OF_CONDUCT.md). Questions and design
chat belong on [Discord](https://discord.gg/9AjGBjGp6Q).

## Support

- [Discord](https://discord.gg/9AjGBjGp6Q) — community and questions
- [Patreon](https://www.patreon.com/c/slabworks) — support the project

## License

This project is open source and licensed under the [MIT License](LICENSE).

## Contributors

Thanks to all our contributors!

[![](https://contrib.rocks/image?repo=slabworks/arenacoach-client)](https://github.com/slabworks/arenacoach-client/graphs/contributors)
