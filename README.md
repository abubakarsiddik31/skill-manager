<div align="center">

<img src="public/logo.svg" width="88" height="88" alt="Skill Manager logo" />

# Skill Manager

### One dashboard to manage every AI coding agent skill you've installed.

Discover, enable, disable, edit, and delete [Agent Skills](#what-is-a-skill)
across Claude Code, Codex, Cursor, Gemini CLI, VS Code, Crush, Roo Code,
Kiro, Junie, Factory Droid, OpenCode — and every tool that reads the
shared `~/.agents/skills` folder (Goose, Amp, …) — without hand-editing
a config file ever again.

[![License: MIT](https://img.shields.io/badge/license-MIT-000000.svg)](LICENSE)
[![Built with Tauri](https://img.shields.io/badge/Tauri-000000.svg?logo=tauri&logoColor=FFC131)](https://tauri.app)
[![Rust](https://img.shields.io/badge/Rust-000000.svg?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![TypeScript](https://img.shields.io/badge/TypeScript-000000.svg?logo=typescript&logoColor=3178C6)](https://www.typescriptlang.org/)
[![React](https://img.shields.io/badge/React-000000.svg?logo=react&logoColor=61DAFB)](https://react.dev/)
[![Latest release](https://img.shields.io/github/v/release/abubakarsiddik31/skill-manager?color=000000&label=release)](https://github.com/abubakarsiddik31/skill-manager/releases/latest)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-000000.svg)](#download)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-000000.svg)](CONTRIBUTING.md)

[**Website**](https://abubakarsiddik31.github.io/skill-manager/) ·
[**Download**](https://github.com/abubakarsiddik31/skill-manager/releases/latest) ·
[**Report a bug**](https://github.com/abubakarsiddik31/skill-manager/issues/new?template=bug_report.yml) ·
[**Contributing**](CONTRIBUTING.md)

<br />

<img src="docs/assets/demo.gif" width="760" alt="Skill Manager demo: browsing, filtering, searching, and viewing a skill across Claude Code, Codex, Cursor, Gemini CLI, and OpenCode" />

</div>

---

## Contents

- [The problem](#the-problem)
- [What is a "skill"?](#what-is-a-skill)
- [Features](#features)
- [Supported tools](#supported-tools)
- [Download](#download)
- [Development](#development)
- [Project structure](#project-structure)
- [Roadmap](#roadmap)
- [Contributing](#contributing)
- [FAQ](#faq)
- [License](#license)

## The problem

If you use more than one AI coding agent, your skills are scattered across
`~/.claude/skills`, `~/.agents/skills`, `~/.cursor/skills`,
`~/.gemini/skills`, `~/.copilot/skills`, `~/.roo/skills`, and more —
eleven folders, zero shared view.

Left unchecked, this turns into **skill hell**: every skill you've ever
installed sits there active, and your agent has to disambiguate against all
of them just to pick the right one for what you're doing right now. A
crowded skills directory doesn't just look messy — the more skills compete
for a match, the worse an agent gets at triggering the right one. Disabling
what you don't need for the current project is how you climb back out.

| | Without Skill Manager | With Skill Manager |
| --- | --- | --- |
| **See what's installed** | Open a terminal, `ls` eleven different folders | One dashboard, every tool |
| **Turn a skill off** | Delete the folder and hope you don't need it back | Toggle it — fully reversible |
| **Keep triggering accurate** | Every installed skill competes for a match, even ones irrelevant to this project | Disable the noise, leave only what's relevant enabled |
| **Know what's wired into a project** | Global and project-level skills look identical until something breaks | Per-project breakdown, separate from global |
| **Edit a `SKILL.md`** | Open a text editor, hunt down the path | Rendered markdown with one-click raw edit |

**Skill Manager fixes all of it.** One dashboard, every tool.

## What is a "skill"?

Anthropic introduced **[Agent Skills](https://code.claude.com/docs/en/skills)**
— folders containing a `SKILL.md` file with instructions an AI coding agent
can load on demand — and the idea has since been picked up across the
ecosystem (Claude Code, Codex, Cursor, Gemini CLI, OpenCode, and others).

## Features

- 🗂️ **Unified view** across every tool's skills directory — no more digging
  through a dozen different config folders by hand.
- 🔀 **Enable / disable** any skill without deleting it (skills move to a
  sibling `.disabled/` folder — fully reversible, and it never touches the
  tool's own config).
- 📝 **View & edit** — a skill's `SKILL.md` renders as formatted markdown by
  default, with a one-click switch to raw edit.
- 🗑️ **Delete** skills you no longer need.
- 📁 **Per-project breakdown** — track individual project folders and see
  exactly which skills are wired into each one, separate from your global
  skills.
- 🧭 **Detected projects** — the app finds folders you already work in
  (Claude Code history, editor recents, git repos) and offers them as
  one-click suggestions in the sidebar and in a searchable picker, with
  activity times and skill counts on every row.
- 📌 **Pinned & most-used first** — pin the tools and projects you touch
  daily; the sidebar keeps your top three most-used projects (30-day
  window) on top, and every project list can sort by use or by skill
  count.
- 🔍 **Search** across names and descriptions.

## Supported tools

| Tool | User-level directory | Project-level directory | Status |
| --- | --- | --- | --- |
| [Claude Code](https://claude.com/claude-code) | `~/.claude/skills` | `.claude/skills` | ✅ fully supported |
| [Agents (shared)](https://agentskills.io) — read by Codex, Goose, Amp, and others | `~/.agents/skills` | `.agents/skills` | ✅ fully supported |
| [Cursor](https://cursor.com/docs/skills) | `~/.cursor/skills` | `.cursor/skills` | ✅ fully supported |
| [Gemini CLI](https://geminicli.com/docs/cli/skills/) | `~/.gemini/skills` | `.gemini/skills` | ✅ fully supported |
| [VS Code / Copilot](https://code.visualstudio.com/docs/copilot/customization/agent-skills) | `~/.copilot/skills` | `.github/skills` | ✅ fully supported |
| [Crush](https://github.com/charmbracelet/crush) | `~/.config/crush/skills` | `.crush/skills` | ✅ fully supported |
| [Roo Code](https://docs.roocode.com/features/skills) | `~/.roo/skills` | `.roo/skills` | ✅ fully supported |
| [Kiro](https://kiro.dev/docs/skills/) | `~/.kiro/skills` | `.kiro/skills` | ✅ fully supported |
| [Junie](https://junie.jetbrains.com/docs/agent-skills.html) | `~/.junie/skills` | `.junie/skills` | ✅ fully supported |
| [Factory Droid](https://docs.factory.ai/cli/configuration/skills) | `~/.factory/skills` | `.factory/skills` | ✅ fully supported |
| [OpenCode](https://opencode.ai/docs/skills/) | `~/.config/opencode/skills` | `.opencode/skills` | ✅ fully supported |

All eleven paths are verified against each tool's own docs — not guessed. If a
tool changes its convention, [open an issue](.github/ISSUE_TEMPLATE/tool_directory.yml)
and we'll update the adapter.

The sidebar itself is tool-level: each tool's view shows every folder it
reads, skills in the shared `~/.agents/skills` folder carry a "seen by" chip
for every tool that discovers them, and toggling one warns that it affects
all of them — there's a single copy on disk.

## Download

Grab a build from the [latest release](https://github.com/abubakarsiddik31/skill-manager/releases/latest):

| Platform | File |
| --- | --- |
| macOS (Apple Silicon + Intel) | `.dmg` (universal) |
| Windows | `.exe` or `.msi` |
| Linux | `.deb`, `.rpm`, or `.AppImage` |

> [!NOTE]
> **Builds are currently unsigned.** We're working on code-signed releases
> (see [Roadmap](#roadmap)) so this warning goes away — until then, here's
> how to open the app anyway:
>
> **macOS** — Gatekeeper will say *"Skill Manager" Not Opened* / *Apple could
> not verify "Skill Manager" is free of malware*. Click **Done**, then either:
> - Right-click (Control-click) the app in Finder → **Open** → **Open** in
>   the dialog that appears, or
> - Go to **System Settings → Privacy & Security**, scroll down to the
>   blocked-app notice, and click **Open Anyway** → **Open Anyway** again to
>   confirm.
>
> You only need to do this once — it launches normally after that.
>
> **Windows** — SmartScreen will show *Windows protected your PC*. Click
> **More info**, then **Run anyway**.

## Development

Requires [Node.js](https://nodejs.org/) and the
[Rust toolchain](https://www.rust-lang.org/tools/install) (Tauri's
prerequisites are documented [here](https://tauri.app/start/prerequisites/)).

```bash
git clone https://github.com/abubakarsiddik31/skill-manager.git
cd skill-manager
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
(`src-tauri/src/skills/mod.rs`), so adding support for a new tool is just a
new module pointing at its skills directory.

## Roadmap

- [ ] Skill browser & install — browse community skills and install them
  into any tool's skills folder in one click
- [ ] Auto-update support (in-app update checks, no manual reinstall)
- [ ] Code-signed builds (no more Gatekeeper/SmartScreen warnings)
- [ ] Keep adding agents as they adopt `SKILL.md` — Windsurf and Trae are on
  the radar, but their conventions aren't verified yet

See [CONTRIBUTING.md](CONTRIBUTING.md) for how to pick any of these up, or add
support for a tool not listed above.

## Contributing

This project gets meaningfully better with a handful of small, specific
contributions — see [CONTRIBUTING.md](CONTRIBUTING.md) for exactly how to:

- Pin down a tool's real skills directory (the single biggest gap right now)
- Add support for an entirely new coding agent
- Report a bug or open a fix

Every one of these is scoped to be doable in one sitting. Issues and PRs
welcome. Please also read our [Code of Conduct](CODE_OF_CONDUCT.md).

## FAQ

**Does this modify how my tools load skills?**
No. Skill Manager only moves folders between a skills directory and a sibling
`.disabled/` folder, and edits `SKILL.md` files directly on disk. It never
touches a tool's own config or settings files.

**Is it safe to disable a skill?**
Yes — disabling moves the skill folder to `.disabled/` next to it. Re-enabling
moves it back. Nothing is deleted until you explicitly delete it.

**Does it phone home or collect telemetry?**
No. Skill Manager is a local desktop app — it only reads and writes the skill
directories already on your machine.

**Why isn't `<my tool>` supported?**
Open an issue with a link to that tool's own skills documentation and we'll
add an adapter — see [Contributing](#contributing).

## License

[MIT](LICENSE)
