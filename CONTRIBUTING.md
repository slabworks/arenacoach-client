# Contributing

Pull requests are welcome, including work from Claude, Cursor, Codex, and
other agents.

The person who opens the PR owns the change. Keep it focused, tested, and
something you can explain.

## Ways to help

- Fix bugs in the companion window, sign-in, or match sync
- Improve how finished games are read from Arena
- Add tests
- Improve the docs

Talk through large ideas on [Discord](https://discord.gg/9AjGBjGp6Q) first.
Please read the [Code of Conduct](CODE_OF_CONDUCT.md).

## What to keep in mind

- This is post-game review, not a live overlay or “play this card” helper.
- Upload a finished match, not the raw Arena log file.
- Leave player names and account ids off anything that leaves the computer.
- This is unofficial fan content. “MTG Arena” is fine; do not use Wizards marks as the product logo.

The website lives in [arenacoach-web](https://github.com/slabworks/arenacoach-web).

## Setup

You need [Bun](https://bun.sh), a recent stable Rust toolchain, and the
[Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS.

On Windows, install the [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
with “Desktop development with C++”, then install Rust with the MSVC toolchain
(`x86_64-pc-windows-msvc` or `aarch64-pc-windows-msvc`). WebView2 is already on
current Windows 10/11.

These commands work in PowerShell, Command Prompt, or a Unix shell:

```
git clone https://github.com/slabworks/arenacoach-client.git
cd arenacoach-client
bun install
```

`bun run desktop` is the real app (needed to read Arena). `bun run dev` is
window UI only.

## Before you open a PR

```
bun test
bun run build
cargo test --manifest-path src-tauri/Cargo.toml
```

1. Fork the repository.
2. Branch from `main`.
3. Make a focused change.
4. Add or update tests when behavior changes.
5. Open a pull request that says what changed and why.

Screenshots help for window or layout work.

## Releases

Pushes to `main` that pass tests publish Windows and macOS installers to
the [latest GitHub release](https://github.com/slabworks/arenacoach-client/releases/latest).
You can also run the `release` workflow by hand.
