## Contributing

Skill Manager is small on purpose — most useful contributions fall into one of
these buckets. Please read our [Code of Conduct](CODE_OF_CONDUCT.md) before
participating.

### Keep a tool's skill directory in sync

Claude Code, Codex, Cursor, Gemini CLI, and OpenCode's skill directories are
all verified
against each tool's own docs (see the [README](README.md#supported-tools)),
but these tools evolve. If a tool changes its convention or you find it's
wrong:

1. Open an issue using the [tool directory correction
   template](https://github.com/abubakarsiddik31/skill-manager/issues/new?template=tool_directory.yml), with a source
   (docs link or changelog entry).
2. Or go straight to a PR: update the path in `src-tauri/src/skills/<tool>.rs`
   (both `skills_dir()` and `project_subpath()`), and the tables in
   `README.md` and `docs/index.html`.

### Add a new tool adapter

Every skills folder implements the same `SkillAdapter` trait
(`src-tauri/src/skills/mod.rs`). Tools themselves live in the registry at
`src-tauri/src/skills/tools.rs` — a tool either owns a folder adapter or
just reads existing ones (like Goose and Amp read the shared
`~/.agents/skills`). To add one:

1. Create `src-tauri/src/skills/<tool>.rs` following the existing adapters as
   a template.
2. Register it in `all_adapters()` and `adapter_for()` in
   `src-tauri/src/skills/mod.rs`.
3. Add the folder to the `AgentTool` enum and its `label()`, and add or
   update the tool's entry in `skills/tools.rs` with every folder it reads
   (`own` vs `compat`).
4. Cite the source you verified the directory paths against, in a doc
   comment — same as the existing adapters.

### Report a bug

Open an issue using the [bug report
template](https://github.com/abubakarsiddik31/skill-manager/issues/new?template=bug_report.yml). Include steps to
reproduce — issues without them are much slower to act on.

### Propose a feature

Open an issue using the [feature request
template](https://github.com/abubakarsiddik31/skill-manager/issues/new?template=feature_request.yml) before writing code
for anything beyond a small fix, so we're aligned on direction first.

### Pull request process

1. Fork the repo and create a branch off `main`.
2. Make your change. Keep the diff scoped to one concern.
3. Run the checks below - both must pass clean.
4. Open a PR with a clear description of *why*, not just *what*. Link the
   issue it resolves, if any.
5. Commit messages: imperative mood, lowercase start (`fix ...`, `add ...`,
   not `Fixed`/`Added`), explain the reasoning in the body when it's not
   obvious from the diff alone.

### Development setup

```bash
npm install
npm run tauri dev
```

Requires the [Rust toolchain](https://www.rust-lang.org/tools/install)
(Tauri's prerequisites are documented
[here](https://tauri.app/start/prerequisites/)).

Run `npx tsc --noEmit` and `cargo check` (from `src-tauri/`) before opening a
PR — both must pass clean. If you touched `src-tauri/src/skills/`, also run
`cargo test` from `src-tauri/`.

## Notes

docs: refresh demo GIF to reflect the v0.3.1 UI

## Summary

