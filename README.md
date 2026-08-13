<div align="center">

<img src="public/logo.svg" width="96" height="96" alt="Skill Manager logo" />

# Skill Manager

**One dashboard to manage every AI coding agent skill you've installed.**

[![License: MIT](https://img.shields.io/badge/license-MIT-22c55e.svg)](LICENSE)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri-22c55e.svg)](https://tauri.app)
[![TypeScript](https://img.shields.io/badge/TypeScript-strict-22c55e.svg)](https://www.typescriptlang.org/)
[![Latest release](https://img.shields.io/github/v/release/abubakarsiddik31/skill-manager?color=22c55e&label=release)](https://github.com/abubakarsiddik31/skill-manager/releases/latest)

[**Website**](https://abubakarsiddik31.github.io/skill-manager/) · [**Download**](https://github.com/abubakarsiddik31/skill-manager/releases/latest)

</div>

---

## The problem

If you use more than one AI coding agent, your skills are scattered across
`~/.claude/skills`, `~/.codex/skills`, `~/.cursor/skills`, and
`~/.config/opencode/skills` — with no shared way to see what's installed, what it
does, or whether it's even active. Disabling one means deleting it or
hand-editing config. There's no view across tools, and no per-project breakdown
of what's actually wired into a given codebase.

**Skill Manager fixes that.** One dashboard, every tool, full control: discover,
enable, disable, edit, and delete skills for
[Claude Code](https://claude.com/claude-code), Codex, Cursor, and OpenCode —
without touching a config file by hand.

## What is a "skill"?

Anthropic introduced **Agent Skills** — folders containing a `SKILL.md` file with
instructions an AI coding agent can load on demand — and the idea has since been
picked up across the ecosystem (Claude Code, Codex, Cursor, OpenCode, and others).

## Features

- **Unified view** across every tool's skills directory — no more digging through
  `~/.claude/skills`, `~/.codex/skills`, `~/.cursor/skills`, and
  `~/.config/opencode/skills` by hand.
- **Enable / disable** any skill without deleting it (skills are moved to a
  sibling `.disabled/` folder — fully reversible, and it never touches the tool's
  own config).
- **Edit** a skill's `SKILL.md` directly from the app.
- **Delete** skills you no longer need.
- **Per-project breakdown** — track individual project folders and see the
  `.claude/skills`, `.codex/skills`, etc. installed inside each one, separate
  from your global (user-level) skills.
- **Search** across names and descriptions.
- Dark, terminal-inspired UI — built for people who live in a monospace font.

## Supported tools

| Tool | Skills directory | Status |
| --- | --- | --- |
| Claude Code | `~/.claude/skills` | ✅ fully supported |
| Codex | `~/.codex/skills` | 🧪 best-effort (path unconfirmed) |
| Cursor | `~/.cursor/skills` | 🧪 best-effort (path unconfirmed) |
| OpenCode | `~/.config/opencode/skills` | 🧪 best-effort (path unconfirmed) |

Claude Code's `SKILL.md` convention is well documented and fully wired up. The
other three tools' skill directories are evolving and not yet officially
standardized — if you know the real convention for one of them, a PR updating
`src-tauri/src/skills/<tool>.rs` is very welcome.

## Download

Grab a build from the [latest release](https://github.com/abubakarsiddik31/skill-manager/releases/latest):

| Platform | File |
| --- | --- |
| macOS (Apple Silicon + Intel) | `.dmg` (universal) |
| Windows | `.exe` or `.msi` |
| Linux | `.deb`, `.rpm`, or `.AppImage` |

Builds are unsigned — macOS Gatekeeper and Windows SmartScreen will warn on
first launch (right-click → Open on macOS, "More info" → "Run anyway" on
Windows).

## Development

Requires [Node.js](https://nodejs.org/) and the
[Rust toolchain](https://www.rust-lang.org/tools/install) (Tauri's prerequisites
are documented [here](https://tauri.app/start/prerequisites/)).

```bash
npm install
npm run tauri dev
```

To build a release binary for your platform:

```bash
npm run tauri build
```

## Project structure

```
src/                    react + typescript frontend
  lib/api.ts            thin client over the tauri commands
  types.ts              shared frontend types
src-tauri/src/
  skills/               one adapter per tool, all implementing SkillAdapter
  commands.rs           tauri commands exposed to the frontend
```

Each tool implements the same `SkillAdapter` trait
(`src-tauri/src/skills/mod.rs`), so adding support for a new tool is just a new
module pointing at its skills directory.

## Contributing

Issues and PRs welcome — especially ones that pin down the real skills-directory
convention for Codex, Cursor, or OpenCode, or that add a new adapter entirely.

## License

[MIT](LICENSE)
