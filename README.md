# Arena Coach companion

Unofficial desktop companion for Magic: The Gathering Arena. After you finish
a game, it syncs that match to [Arena Coach](https://github.com/slabworks/arenacoach-web)
so you can read private coaching notes.

Not affiliated with Wizards of the Coast.

## What it does

- Reads the Arena log on your computer after you turn on Detailed Logs
- Syncs finished games to your Arena Coach account
- Shows watch status, the latest match, and your notes
- Leaves names and account ids off the upload
- Never uploads the raw Arena log file

## Download

The latest Windows and macOS installers are on the
[Releases](https://github.com/slabworks/arenacoach-client/releases/latest)
page. CI rebuilds that release from `main` after tests pass.

- Windows: `.exe` setup installer
- macOS Apple Silicon: `aarch64` `.dmg`
- macOS Intel: `x64` `.dmg`

macOS may ask you to allow the app in **System Settings → Privacy & Security**
the first time.

## Use it

1. Create an account on the Arena Coach website.
2. In Arena, turn on **Options → Account → Detailed Logs (Plugin Support)** and restart the client.
3. Open the companion, sign in, and leave it running while you play.
4. After a game, read your notes in the companion or on the website.

If the companion cannot find the game file, Detailed Logs are probably still
off, or Arena needs another restart.

## Build from source

You need [Bun](https://bun.sh), a recent stable [Rust](https://rustup.rs)
toolchain, and the [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/)
for your OS. On Windows, that is the Microsoft C++ Build Tools (“Desktop
development with C++”), the MSVC Rust toolchain, and WebView2 (already on
current Windows 10/11).

These commands work in PowerShell, Command Prompt, or a Unix shell:

```
git clone https://github.com/slabworks/arenacoach-client.git
cd arenacoach-client
bun install
bun run desktop
```

`bun run desktop` is the real app. `bun run dev` is only the window UI and
cannot read Arena.

Settings (gear button):

- **Developer mode** talks to the local website instead of production. Changing it signs you out.
- **Show debug info** adds connection and file-path details.
- **Replay fixture match** is only in developer mode.

Arena’s log is usually:

- macOS: `~/Library/Logs/Wizards Of The Coast/MTGA/Player.log`
- Windows: `%USERPROFILE%\AppData\LocalLow\Wizards Of The Coast\MTGA\Player.log`

## Tests

```
bun test
bun run build
cargo test --manifest-path src-tauri/Cargo.toml
```

## Contributing

Pull requests are welcome, including work from Claude, Cursor, Codex, and
other agents. See [CONTRIBUTING.md](CONTRIBUTING.md).

Please follow the [Code of Conduct](CODE_OF_CONDUCT.md). Chat is on
[Discord](https://discord.gg/9AjGBjGp6Q).

## Support

- [Discord](https://discord.gg/9AjGBjGp6Q)
- [Patreon](https://www.patreon.com/c/slabworks)

## License

MIT. See [LICENSE](LICENSE).

## Contributors

Thanks to everyone who helps.

[![](https://contrib.rocks/image?repo=slabworks/arenacoach-client)](https://github.com/slabworks/arenacoach-client/graphs/contributors)
