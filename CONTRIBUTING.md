# Contributing

Contributions are welcome through pull requests. That includes work written
with Claude, Cursor, Codex, and other agents.

The person who opens the PR owns the change: keep it focused, tested, and
something you can explain in review.

## Ways to help

- Fix bugs and improve the log watcher, match assembler, or desktop UI
- Add tests around fixture logs and upload behavior
- Improve docs and onboarding
- Talk through ideas on [Discord](https://discord.gg/9AjGBjGp6Q) before a large PR

Please read the [Code of Conduct](CODE_OF_CONDUCT.md).

## Agentic code

Agent PRs are welcome. Before you open one:

1. Read [README.md](README.md) and [AGENTS.md](AGENTS.md).
2. Stay inside the product constraints below.
3. Run the validation commands and keep the suite green.
4. Do not open drive-by refactors, comment-only diffs, or license/header churn.

## Product constraints

- Post-game review only. No live “play this card” overlay.
- Never upload a raw `Player.log`. The device posts a parsed match payload.
- Strip identity fields (screen name, user id, session id, opponent name) before the request leaves the machine.
- Model API keys stay on the web server, never in this app.
- Unofficial fan content. Nominative “MTG Arena” is fine; do not use Wizards marks in the product lockup.

The web app lives in [`arenacoach-web`](https://github.com/slabworks/arenacoach-web).

## Setup

You need [Bun](https://bun.sh), a recent stable Rust toolchain, and the Tauri 2
system libraries for your OS.

```bash
git clone https://github.com/slabworks/arenacoach-client.git
cd arenacoach-client
bun install
```

## Validation

Run these before you open a PR:

```bash
bun test
bun run build
cargo test --manifest-path src-tauri/Cargo.toml
```

## Pull requests

1. Fork the repository.
2. Create a feature branch from `main`.
3. Make a focused change with a clear commit message.
4. Add or update tests for any behavior change.
5. Open a pull request that says what changed and why.

Use the PR template. Screenshots help for UI work.
