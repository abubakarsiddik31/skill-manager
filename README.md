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
`~/.claude/skills`, `~/.agents/skills`, `~/.cursor/skills`, and
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

| Tool | User-level directory | Project-level directory | Status |
| --- | --- | --- | --- |
| Claude Code | `~/.claude/skills` | `.claude/skills` | ✅ fully supported |
| Codex | `~/.agents/skills` | `.agents/skills` | ✅ fully supported |
| Cursor | `~/.cursor/skills` | `.cursor/skills` | ✅ fully supported |
| OpenCode | `~/.config/opencode/skills` | `.opencode/skills` | ✅ fully supported |

All four paths are verified against each tool's own docs — [Claude
Code](https://code.claude.com/docs/en/skills),
[Codex](https://developers.openai.com/codex/skills),
[Cursor](https://cursor.com/docs/skills), and
[OpenCode](https://opencode.ai/docs/skills/) — not guessed. If a tool changes
its convention, [open an issue](.github/ISSUE_TEMPLATE/tool_directory.yml) and
we'll update the adapter.

## Roadmap

With all four tools verified, the focus moves to:

- [ ] Add Gemini / Google AI support
- [ ] Add support for other emerging coding agents as they adopt `SKILL.md`
- [ ] Auto-update support (in-app update checks, no manual reinstall)
- [ ] Code-signed builds (no more Gatekeeper/SmartScreen warnings)

See [CONTRIBUTING.md](CONTRIBUTING.md) for how to pick any of these up, or add
support for a tool not listed above.

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
