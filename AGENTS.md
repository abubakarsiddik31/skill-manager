# Agent instructions — Skill Manager

Stack: [Tauri v2](https://tauri.app) (Rust backend) + React + TypeScript
frontend, built with Vite.

## Structure

```
src/
  api/                  the invoke client over Rust commands (skills.ts, projects.ts)
  components/
    layout/             app chrome (Sidebar, Topbar)
    skills/             skill grid (SkillList, SkillCard, SortToggle)
    modals/             the four dialogs (Editor, CreateSkill, AddProject, Projects)
    ui/                 shared primitives (ModalShell, icons)
  hooks/                data + mutations (useGlobalSkills, useProjects, useProjectSkills)
  utils/                pure helpers + tests (filtering, sidebar lists, time, markdown)
  types/                shared frontend types split by domain, barrel at types/index.ts
src-tauri/src/
  commands/             tauri commands split by domain — skills.rs, create_skill.rs,
                        projects.rs; mod.rs holds the shared managed-path validation
                        (see the security note below)
  skills/               one adapter per skills folder (claude, agents, copilot, ...),
                        plus tools.rs — the tool→folder registry driving the sidebar
  projects.rs           persisted list of tracked project folders
  detect.rs             project-folder discovery for the add-project picker
docs/                   the landing page (GitHub Pages, docs/index.html)
                        and specs/plans under docs/superpowers/
```

Each skills folder implements the same `SkillAdapter` trait
(`src-tauri/src/skills/mod.rs`); tools are readers listed in
`src-tauri/src/skills/tools.rs`. See
[CONTRIBUTING.md](CONTRIBUTING.md) for how to add a new adapter or fix a
tool's skills-directory path.

## Commands

```bash
npm install
npm run tauri dev        # dev app
npm run tauri build      # release binary for your platform
npx tsc --noEmit         # typecheck — must pass clean before a PR
npm test                 # vitest unit tests (utils helpers) — must pass before a PR
cargo check              # from src-tauri/ — must pass clean before a PR
cargo fmt                # from src-tauri/ — run before committing; no diff should remain
cargo clippy --all-targets   # from src-tauri/ — must pass warning-free before a PR
cargo test               # from src-tauri/ — required if you touched src-tauri/src/
```

Security note: file-touching commands (`read_skill_content`,
`write_skill_content`, `delete_skill`, `set_skill_enabled`) validate
their paths against the skills roots the app manages (`skills_roots` /
`validate_manifest_at` in `src-tauri/src/commands/mod.rs`). Never
bypass that check when adding commands — the webview is untrusted
input like any other frontend.

## Release process

Version is duplicated across five files and the landing page has hardcoded
download links — none of this is automated, all of it must be updated by
hand every release:

1. Bump the version number in:
   - `package.json` (`version`)
   - `package-lock.json` (top-level `version` **and** `packages[""].version`)
   - `src-tauri/tauri.conf.json` (`version`)
   - `src-tauri/Cargo.toml` (`version`)
   - then run `cargo update -p skill-manager` from `src-tauri/` to refresh
     `src-tauri/Cargo.lock` (don't hand-edit the lockfile).
2. Update `docs/index.html`:
   - the three hardcoded asset URLs under `<div class="downloads" id="downloads">`
     (`releases/download/vX.Y.Z/Skill.Manager_X.Y.Z_...`) for macOS, Windows,
     Linux
   - the `vX.Y.Z · free · open source · mit licensed ·` version text right
     below the download buttons
3. Commit the version bump, push to `main`.
4. `git tag vX.Y.Z && git push origin vX.Y.Z` — this triggers
   `.github/workflows/release.yml`, which builds macOS (universal), Windows,
   and Linux via `tauri-action` and creates a **draft** GitHub release with
   the installers attached.
5. Once the workflow succeeds, publish the draft so it becomes the public
   "latest" release and the download links on the site actually resolve:
   `gh release edit vX.Y.Z --draft=false`.

Builds are unsigned (see README's Download section) — this is expected,
code signing is on the roadmap, not a bug to fix silently.

## Conventions

- No `Co-Authored-By` trailer on commits in this repo.
- Commit messages: imperative mood, lowercase start (`fix ...`, `add ...`),
  explain *why* in the body when it's not obvious from the diff.
- Keep PRs scoped to one concern (see [CONTRIBUTING.md](CONTRIBUTING.md)).
