# Layered structure tidy-up — design

Date: 2026-08-21
Status: approved (approach chosen from three options: layered tidy-up over
feature-sliced FE and Rust-only)

## Problem

After PR #11 the codebase has outgrown its flat layout:

- `src-tauri/src/commands.rs` is 648 lines mixing four concerns: the
  path-security core, skill CRUD commands, the whole skill-creation
  feature (name policy + YAML templating), and the project commands —
  plus ~265 lines of tests interleaved.
- `src/components/` is a flat grab-bag: layout, skill widgets, and four
  modals sit side by side.
- Four modals each re-implement the same overlay pattern (window Escape
  listener, `.modal-overlay` click-to-close, `.modal` stopPropagation)
  and the same close-X SVG; `PinIcon` lives inline in `Sidebar`.
- `src/lib/` mixes the Tauri API client with pure helpers.
- `src/types.ts` is one file spanning tool, skill, and project domains.

## Goals

- Every file lands in a folder named for its role; a new contributor
  can guess where a change goes without reading the tree first.
- The security-critical path validation becomes a named module rather
  than a section of a 648-line file.
- Pure refactor: no exported symbol renames, no behavior change, no CSS
  edits. Same class names, same wire format, same tests (moved, not
  rewritten).

## Non-goals

- Splitting `App.css` (988 lines, untouched).
- Fixing `App.tsx` prop drilling / introducing context (architecture,
  not structure).
- Splitting `skills/mod.rs` internals or deduping the adapter registry.
- tsconfig path aliases (imports stay relative, max two levels up).

## Frontend

Target layout:

```
src/
  api/
    skills.ts       invoke wrappers: listToolEntries, listSkills,
                    setSkillEnabled, deleteSkill, readSkillContent,
                    writeSkillContent, createSkill
    projects.ts     listProjects, listDetectedProjects,
                    refreshDetectedProjects, addProject, removeProject,
                    setProjectPinned, touchProject,
                    listProjectSkillCounts, listProjectSkills,
                    pickProjectFolder
    index.ts        export const api = { ...skillsApi, ...projectApi }
  components/
    layout/         Sidebar.tsx, Topbar.tsx
    skills/         SkillList.tsx, SkillCard.tsx, SortToggle.tsx
    modals/         EditorModal.tsx, CreateSkillModal.tsx,
                    AddProjectModal.tsx, ProjectsModal.tsx
    ui/             ModalShell.tsx (new), icons.tsx (new)
  hooks/            unchanged
  utils/            filterSkills.ts, relativeTime.ts, projectUsage.ts,
                    sidebarLists.ts, markdown.ts, __tests__/
  types/
    tool.ts         AgentTool, FolderRole, ToolFolderInfo, ToolEntry
    skill.ts        SkillScope, Skill
    project.ts      ProjectInfo, DetectedProject
    view.ts         View
    index.ts        barrel re-export of the four files
```

Churn-limiting decisions:

- `types/index.ts` keeps every existing `from "../types"` import
  resolving — consumers of types change nothing.
- The composed `api` object keeps call sites unchanged; only the import
  path moves (`../lib/api` → `../api`), one line per consumer.
- 1:1 file moves use `git mv`; only `types.ts` and `lib/api.ts` are
  genuine splits (history there is accepted as rewritten).

### New code — `components/ui/ModalShell.tsx`

```tsx
interface ModalShellProps {
  /** extra class on the .modal div, e.g. "create-modal" */
  className?: string;
  onClose: () => void;
  children: React.ReactNode;
}
```

Owns the pattern currently duplicated in all four modals: a
`useEffect` window keydown listener closing on Escape (with cleanup),
`<div className="modal-overlay" onClick={onClose}>`, and an inner
`<div className={`modal ${className ?? ""}`} onClick={(e) => e.stopPropagation()}>`.
Each modal drops its own copy and keeps its header/footer/body markup.

### New code — `components/ui/icons.tsx`

```tsx
export function PinIcon({ size = 12 }: { size?: number })
export function CloseIcon({ size = 14 }: { size?: number })
```

SVGs copied verbatim from `Sidebar.tsx` and the modal headers; the only
change is the width/height attribute taking `size`.

## Backend

`src-tauri/src/commands.rs` becomes `src-tauri/src/commands/`:

```
commands/
  mod.rs          shared core: MANIFEST_FILE, DISABLED_DIR,
                  skills_roots, validate_manifest_at (doc comments kept
                  verbatim — this is the security check AGENTS.md warns
                  never to bypass), manifest_is_manageable,
                  find_skill_by_manifest; `mod`/`pub use` wiring for the
                  submodules; tests for skills_roots + validate_manifest_at
                  (shape, symlink, lookalike roots)
  skills.rs       list_tool_entries, list_skills, set_skill_enabled,
                  delete_skill, read_skill_content, write_skill_content
  create_skill.rs create_skill + WINDOWS_RESERVED_NAMES,
                  validate_skill_folder_name, yaml_double_quoted,
                  create_skill_manifest; the seven name-validation and
                  creation tests
  projects.rs     the nine project commands + ProjectSkillCount struct
```

- Private items in `mod.rs` stay private — Rust allows child modules to
  reach them via `super::`, so no visibility widening beyond the
  existing `pub` on `#[tauri::command]` functions.
- `lib.rs` is untouched: `commands::list_skills` et al. resolve through
  the re-exports, so `generate_handler!` paths are unchanged.
- `skills/`, `detect.rs`, `projects.rs` untouched.

## Documentation

`AGENTS.md` structure section is rewritten to describe the new tree
(`components/` subfolders, `api/`, `utils/`, `types/`, `commands/`
module dir). The security note is updated to name
`src-tauri/src/commands/mod.rs` as where the validation lives.

## Verification gate (must all pass, per AGENTS.md)

1. `npx tsc --noEmit`
2. `npm test` (tests move with the helpers to `src/utils/__tests__/`)
3. `cargo check`, `cargo fmt` (no diff), `cargo clippy --all-targets`,
   `cargo test` — from `src-tauri/`; test counts before and after must
   match (same tests, new module paths)
4. Boot `npm run tauri dev`: sidebar renders, open a project, open the
   editor, create a skill, Escape closes each modal.

## Risks

- Splitting a file can orphan a helper (clippy/rustc dead-code warnings)
  — mitigation: each helper moves to the module that calls it.
- Vitest include globs are the default (`**/*.test.*`), so moving
  `__tests__/` is safe; verify during implementation anyway.
- `git mv` detection on renames with import edits: keep moves and
  content edits in separate commits per area so history stays readable.
