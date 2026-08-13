<div align="center">

<img src="public/logo.svg" width="96" height="96" alt="Skill Manager logo" />

# Skill Manager

**One dashboard to manage every AI coding agent skill you've installed.**

[![License: MIT](https://img.shields.io/badge/license-MIT-000000.svg)](LICENSE)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri-000000.svg)](https://tauri.app)
[![TypeScript](https://img.shields.io/badge/TypeScript-strict-000000.svg)](https://www.typescriptlang.org/)
[![Latest release](https://img.shields.io/github/v/release/abubakarsiddik31/skill-manager?color=000000&label=release)](https://github.com/abubakarsiddik31/skill-manager/releases/latest)

[**Website**](https://abubakarsiddik31.github.io/skill-manager/) · [**Download**](https://github.com/abubakarsiddik31/skill-manager/releases/latest) · [**Contributing**](CONTRIBUTING.md)

</div>

---

## The problem

If you use more than one AI coding agent, your skills are scattered across
`~/.claude/skills`, `~/.codex/skills`, `~/.cursor/skills`, and
`~/.config/opencode/skills` — four folders, four formats, zero shared view.

- **You can't see what's installed.** Every tool hides its skills in its own
  config folder, so "what do I have?" means opening a terminal and grepping.
- **You can't turn one off.** None of these tools ship a disable switch — your
  only option is deleting the folder and hoping you don't need it back.
- **You can't see what's wired into a project.** A skill installed globally and
  one dropped into a repo's `.claude/skills` look identical until something
  breaks.

**Skill Manager fixes all three.** One dashboard, every tool: discover, enable,
disable, edit, and delete skills for
[Claude Code](https://claude.com/claude-code), Codex, Cursor, and OpenCode —
without touching a config file by hand.

## What is a "skill"?

Anthropic introduced **Agent Skills** — folders containing a `SKILL.md` file with
instructions an AI coding agent can load on demand — and the idea has since been
picked up across the ecosystem (Claude Code, Codex, Cursor, OpenCode, and others).

## Features

- **Unified view** across every tool's skills directory — no more digging through
  four different config folders by hand.
- **Enable / disable** any skill without deleting it (skills are moved to a
  sibling `.disabled/` folder — fully reversible, and it never touches the tool's
  own config).
- **View & edit** — a skill's `SKILL.md` renders as formatted markdown by
  default, with a one-click switch to raw edit.
- **Delete** skills you no longer need.
- **Per-project breakdown** — track individual project folders and see exactly
  which skills are wired into each one, separate from your global skills.
- **Search** across names and descriptions.

## Supported tools

| Tool | Skills directory | Status |
| --- | --- | --- |
| Claude Code | `~/.claude/skills` | ✅ fully supported |
| Codex | `~/.codex/skills` | 🧪 best-effort (path unconfirmed) |
| Cursor | `~/.cursor/skills` | 🧪 best-effort (path unconfirmed) |
| OpenCode | `~/.config/opencode/skills` | 🧪 best-effort (path unconfirmed) |

Claude Code's `SKILL.md` convention is well documented and fully wired up. The
other three tools' skill directories are evolving and not yet officially
standardized — see the roadmap below.

## Roadmap

The near-term goal is **100% verified support for the four tools above** before
expanding further:

- [ ] Confirm and verify Codex's real skills directory convention
- [ ] Confirm and verify Cursor's real skills directory convention
- [ ] Confirm and verify OpenCode's real skills directory convention
- [ ] Mark all four tools "fully supported" in the table above

Once that's done:

- [ ] Add Gemini / Google AI support
- [ ] Add support for other emerging coding agents as they adopt `SKILL.md`
- [ ] Auto-update support (in-app update checks, no manual reinstall)
- [ ] Code-signed builds (no more Gatekeeper/SmartScreen warnings)

If you actually use one of the unconfirmed tools, [this is the single most
useful thing you can contribute](CONTRIBUTING.md#pin-down-a-tools-real-skill-directory) —
it's a one-line fix, and it directly unblocks the roadmap above.

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
src/
  components/           presentational react components (Sidebar, SkillCard, EditorModal, ...)
  hooks/                data + mutations (useGlobalSkills, useProjects, useProjectSkills)
  lib/                  api client, markdown rendering, filtering helpers
  types.ts              shared frontend types
src-tauri/src/
  skills/               one adapter per tool, all implementing SkillAdapter
  projects.rs           persisted list of tracked project folders
  commands.rs           tauri commands exposed to the frontend
docs/                   the landing page (GitHub Pages)
```

Each tool implements the same `SkillAdapter` trait
(`src-tauri/src/skills/mod.rs`), so adding support for a new tool is just a new
module pointing at its skills directory.

## Contributing

This project gets meaningfully better with a handful of small, specific
contributions — see [CONTRIBUTING.md](CONTRIBUTING.md) for exactly how to:

- Pin down a tool's real skills directory (the single biggest gap right now)
- Add support for an entirely new coding agent
- Report a bug or open a fix

Every one of these is scoped to be doable in one sitting. Issues and PRs
welcome.

## License

[MIT](LICENSE)
