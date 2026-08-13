## Contributing

Skill Manager is small on purpose — most useful contributions fall into one of
these buckets.

### Pin down a tool's real skill directory

Codex, Cursor, and OpenCode's skill directory conventions are best-effort
guesses right now (see the table in the [README](README.md#supported-tools)).
If you actually use one of these tools and know its real convention:

1. Update the path in `src-tauri/src/skills/<tool>.rs` (both `skills_dir()`
   and `project_subpath()`).
2. Update the table in `README.md` and `docs/index.html`.
3. Open a PR. One-line path fixes are very welcome.

### Add a new tool adapter

Every tool implements the same `SkillAdapter` trait
(`src-tauri/src/skills/mod.rs`). To add one:

1. Create `src-tauri/src/skills/<tool>.rs` following the existing adapters as
   a template.
2. Register it in `all_adapters()` and `adapter_for()` in
   `src-tauri/src/skills/mod.rs`.
3. Add the tool to the `AgentTool` enum and its `label()`.

### Bug fixes / small improvements

Open an issue first for anything beyond a small fix, so we're aligned before
you put in the work. For small fixes, a PR is fine on its own.

### Development setup

```bash
npm install
npm run tauri dev
```

Requires the [Rust toolchain](https://www.rust-lang.org/tools/install)
(Tauri's prerequisites are documented
[here](https://tauri.app/start/prerequisites/)).

Run `npx tsc --noEmit` and `cargo check` (from `src-tauri/`) before opening a
PR — both must pass clean.
