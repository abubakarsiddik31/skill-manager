See [AGENTS.md](AGENTS.md) for full agent instructions (structure, dev
commands, conventions).

The one thing most worth remembering: **the release process is entirely
manual**. Version is duplicated across `package.json`, `package-lock.json`,
`src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml`/`Cargo.lock`, and
`docs/index.html` has hardcoded per-version download URLs — nothing
regenerates these automatically. See the "Release process" section in
[AGENTS.md](AGENTS.md) for the exact steps and files.
