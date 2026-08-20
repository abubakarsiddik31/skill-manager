# Layered Structure Tidy-Up Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reorganize skill-manager into role-named modules (Rust `commands/` dir, FE `api/`/`utils/`/`types/`/component subgroups) as a pure refactor with zero behavior change.

**Architecture:** Seven independently-green tasks, each ending in a commit on a `refactor/layered-structure` branch. Moves use `git mv` (history survives); only genuinely new composition (`types/` split, `api/` split, `commands/` wiring, `ui/ModalShell`, `ui/icons`) is written fresh. Existing test suites are the spec — TDD test-writing does not apply because no behavior is added or changed.

**Tech Stack:** Tauri v2 (Rust backend), React + TypeScript + Vite frontend, Vitest, cargo check/fmt/clippy/test.

## Global Constraints

- Pure refactor: no exported symbol renames, no behavior change, no CSS edits — same class names everywhere (`modal-overlay`, `modal`, `create-modal`, `add-modal` all stay).
- This is a refactor PR on branch `refactor/layered-structure`, one commit per task, never committed directly to `main`.
- Repo commit conventions (AGENTS.md): imperative mood, lowercase start, no `Co-Authored-By` trailer.
- Every task must leave the tree green before its commit: `npx tsc --noEmit` and `npm test` for FE tasks; `cargo check`, `cargo fmt` (no diff), `cargo clippy --all-targets`, `cargo test` (from `src-tauri/`) for Rust tasks.
- Test counts must be identical before and after each move — same tests, new paths. Baseline at plan time: `cargo test` = 17 passed, `npm test` = 13 passed (run each suite once before starting to record the actual baseline).
- Security invariant: `skills_roots` / `validate_manifest_at` / `manifest_is_manageable` doc comments move verbatim; the path-validation bar they describe must not change.

---

### Task 1: Split `src-tauri/src/commands.rs` into `commands/` module directory

**Files:**
- Rename: `src-tauri/src/commands.rs` → `src-tauri/src/commands/mod.rs` (via `git mv`)
- Create: `src-tauri/src/commands/skills.rs`
- Create: `src-tauri/src/commands/create_skill.rs`
- Create: `src-tauri/src/commands/projects.rs`
- Modify: none elsewhere — `src-tauri/src/lib.rs` is untouched (`commands::list_skills` etc. resolve through re-exports)

**Interfaces:**
- Consumes: current `commands.rs` items, all moved verbatim. Line numbers below refer to the file as of commit `2454b18`.
- Produces: same 16 `#[tauri::command]` pub functions re-exported from `commands` module; private helpers become parent-module items reachable by children via `super::`.

- [ ] **Step 1: Create the branch and record baselines**

```bash
git checkout -b refactor/layered-structure
cd src-tauri && cargo test 2>&1 | tail -3 && cd ..
npm test 2>&1 | tail -4
```

Expected: cargo "test result: ok" with its passed count (record it), vitest "Test Files ... passed" (record it).

- [ ] **Step 2: Pure move — file becomes a directory module**

```bash
mkdir src-tauri/src/commands
git mv src-tauri/src/commands.rs src-tauri/src/commands/mod.rs
cd src-tauri && cargo check && cargo test 2>&1 | tail -3 && cd ..
```

Expected: compiles and all tests pass unchanged (module path `crate::commands` resolves identically).

- [ ] **Step 3: Extract `commands/skills.rs`**

Create `src-tauri/src/commands/skills.rs` containing, moved **verbatim** from `mod.rs` (then delete them from `mod.rs`): `list_tool_entries` (lines 74–77), `list_skills` (79–85), `set_skill_enabled` (87–96), `delete_skill` (98–115), `read_skill_content` (117–123), `write_skill_content` (125–132).

```rust
use super::manifest_is_manageable;
use crate::projects;
use crate::skills::{self, tools::ToolEntry, Skill};
use std::fs;
use std::path::Path;
use tauri::AppHandle;
```

Append the same `#[cfg(test)] mod tests` nothing here — these commands have no dedicated tests. In `mod.rs`, add the module + re-export and let rustc tell you which now-unused imports to drop from `mod.rs`'s `use` block:

```rust
mod skills;
pub use skills::{
    delete_skill, list_skills, list_tool_entries, read_skill_content, set_skill_enabled,
    write_skill_content,
};
```

Run: `cd src-tauri && cargo check`
Expected: clean; any "unused import" warnings name exactly what to remove from `mod.rs` (`ToolEntry`, `Serialize` may become unused there later — remove only what rustc/clippy flags, when flagged).

- [ ] **Step 4: Extract `commands/create_skill.rs`**

Move **verbatim** from `mod.rs`: the `// Skill creation` section header comment block (134–136), `WINDOWS_RESERVED_NAMES` (140–143), `validate_skill_folder_name` with doc comment (145–172), `yaml_double_quoted` with doc comment (174–193), `create_skill_manifest` with doc comment (195–235), `create_skill` command with doc comment (237–273); and from the tests module: `fresh_roots` (545–550), `folder_name_validation_accepts_standard_names` (502–510), `folder_name_validation_rejects_unsafe_names` (512–543), `create_rejects_roots_the_app_does_not_manage` (552–572), `create_writes_minimal_manifest_and_opens_missing_root` (574–595), `create_reports_collisions_without_touching_existing_skill` (597–613), `create_validates_name_and_description_before_writing` (615–628), `multi_line_descriptions_round_trip_through_the_frontmatter` (630–647).

```rust
use super::{find_skill_by_manifest, skills_roots, DISABLED_DIR, MANIFEST_FILE};
use crate::projects;
use crate::skills::{self, Skill};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

// ... moved items above; tests module at the bottom:

#[cfg(test)]
mod tests {
    use super::*;
    // moved test helpers and tests; `demo_project` (mod.rs lines 388–397)
    // is NOT needed here — only fresh_roots moves.
}
```

In `mod.rs` add:

```rust
mod create_skill;
pub use create_skill::create_skill;
```

Run: `cd src-tauri && cargo check && cargo test 2>&1 | tail -3`
Expected: all tests pass; count unchanged from baseline.

- [ ] **Step 5: Extract `commands/projects.rs`**

Move **verbatim** from `mod.rs`: `list_projects` (275–278), `list_detected_projects` with doc comment (280–288), `refresh_detected_projects` with doc comment (290–300), `ProjectSkillCount` struct (302–307), `list_project_skill_counts` with doc comment (309–323), `add_project` (325–328), `remove_project` (330–333), `set_project_pinned` (335–338), `touch_project` (340–343), `list_project_skills` (345–360).

```rust
use crate::detect;
use crate::projects::{self, ProjectInfo};
use crate::skills::{self, Skill};
use serde::Serialize;
use std::path::Path;
use tauri::AppHandle;
```

In `mod.rs` add:

```rust
mod projects;
pub use projects::{
    add_project, list_detected_projects, list_project_skill_counts, list_projects,
    list_project_skills, refresh_detected_projects, remove_project, set_project_pinned,
    touch_project,
};
```

Run: `cd src-tauri && cargo check && cargo test 2>&1 | tail -3`
Expected: green; total test count equals baseline.

- [ ] **Step 6: Finish `commands/mod.rs`**

What remains in `mod.rs`: module decls + `pub use` re-exports (Step 3–5), the shared core moved verbatim — `MANIFEST_FILE`/`DISABLED_DIR` consts (10–11), `skills_roots` with doc comment (13–26), the `validate_manifest_at` doc comment + fn (28–65), `manifest_is_manageable` (67–70), `find_skill_by_manifest` (362–382) — and its tests module keeping: `demo_project` (388–397), `skills_roots_cover_user_dirs_and_project_subpaths` (399–410), `lookalike_paths_are_not_roots` (412–422), `temp_root` (424–431), `accepts_manifests_in_scanner_shapes_only` (433–467), `symlinked_skills_must_resolve_into_a_root` (469–500). Final import block for `mod.rs` (remove anything unused):

```rust
use crate::projects::{self, ProjectInfo};
use crate::skills::{self, Skill};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::AppHandle;
```

Note: `skills::AgentTool`/`SkillScope` no longer appear in `mod.rs` after the moves, so `use crate::skills::{self, Skill};` suffices there; child modules import what they need themselves.

- [ ] **Step 7: Full Rust gate**

```bash
cd src-tauri && cargo check && cargo fmt && cargo clippy --all-targets && cargo test 2>&1 | tail -3 && cd ..
```

Expected: all clean, zero warnings, test count = baseline.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/commands* && git commit -m "refactor(commands): split handlers into a commands/ module directory

commands.rs had grown to 648 lines mixing the managed-path security
core, skill crud, skill creation, and project commands. the security
check (skills_roots/validate_manifest_at) now lives by name in
commands/mod.rs; tests moved with the code they cover."
```

---

### Task 2: Split `src/types.ts` into `src/types/`

**Files:**
- Delete: `src/types.ts`
- Create: `src/types/tool.ts`, `src/types/skill.ts`, `src/types/project.ts`, `src/types/view.ts`, `src/types/index.ts`

**Interfaces:**
- Produces: identical exported type names; `from "../types"` (and `../../types`) keep resolving via `types/index.ts` — zero consumer edits.

- [ ] **Step 1: Create the five files (content moved verbatim from `src/types.ts`)**

`src/types/tool.ts` — lines 1–29 of current types.ts:

```ts
export type AgentTool =
  | "claude"
  | "agents"
  | "copilot"
  | "crush"
  | "cursor"
  | "factory"
  | "gemini"
  | "junie"
  | "kiro"
  | "opencode"
  | "roo";

/** How a tool relates to a skills folder it reads. */
export type FolderRole = "own" | "compat";

export interface ToolFolderInfo {
  tool: AgentTool;
  dir: string;
  role: FolderRole;
  dirExists: boolean;
}

/** A coding agent and every skills folder it reads (own + compat). */
export interface ToolEntry {
  id: string;
  label: string;
  folders: ToolFolderInfo[];
}
```

`src/types/skill.ts`:

```ts
import type { AgentTool } from "./tool";

export type SkillScope = "user" | "project";

export interface Skill {
  id: string;
  tool: AgentTool;
  name: string;
  description: string;
  path: string;
  scope: SkillScope;
  enabled: boolean;
}
```

`src/types/project.ts` — `ProjectInfo` (with its doc comments) and `DetectedProject` (with its doc comment) verbatim from lines 43–65.

`src/types/view.ts`:

```ts
import type { ProjectInfo } from "./project";

export type View = { kind: "global" } | { kind: "project"; project: ProjectInfo };
```

`src/types/index.ts`:

```ts
export * from "./project";
export * from "./skill";
export * from "./tool";
export * from "./view";
```

- [ ] **Step 2: Delete the old file and gate**

```bash
git rm src/types.ts
npx tsc --noEmit && npm test 2>&1 | tail -4
```

Expected: tsc clean with **zero** changes outside `src/types/`; vitest passes (test files' `../../types` imports resolve through the barrel).

- [ ] **Step 3: Commit**

```bash
git add src/types && git commit -m "refactor(types): split types.ts into per-domain modules

tool/skill/project/view files behind an index barrel so existing
\"../types\" imports keep resolving untouched."
```

---

### Task 3: Split `src/lib/api.ts` into `src/api/`

**Files:**
- Delete: `src/lib/api.ts`
- Create: `src/api/skills.ts`, `src/api/projects.ts`, `src/api/index.ts`
- Modify (import path only, one line each): `src/hooks/useGlobalSkills.ts`, `src/hooks/useProjects.ts`, `src/hooks/useProjectSkills.ts`, `src/hooks/useSkillMutations.ts`, `src/components/EditorModal.tsx`, `src/components/CreateSkillModal.tsx`, `src/components/AddProjectModal.tsx`

**Interfaces:**
- Produces: `api` object with the identical 16 methods; only the import specifier changes.

- [ ] **Step 1: Create `src/api/skills.ts`**

Move the skill-related methods verbatim from `lib/api.ts` (20–54), renamed object:

```ts
import { invoke } from "@tauri-apps/api/core";
import type { AgentTool, Skill, SkillScope, ToolEntry } from "../types";

/** Invoke wrappers over the skill commands in src-tauri/src/commands/skills.rs
 *  and create_skill.rs. */
export const skillsApi = {
  listToolEntries(): Promise<ToolEntry[]> {
    return invoke("list_tool_entries");
  },
  listSkills(): Promise<Skill[]> {
    return invoke("list_skills");
  },
  setSkillEnabled(id: string, enabled: boolean): Promise<Skill> {
    return invoke("set_skill_enabled", { id, enabled });
  },
  deleteSkill(id: string): Promise<void> {
    return invoke("delete_skill", { id });
  },
  readSkillContent(id: string): Promise<string> {
    return invoke("read_skill_content", { id });
  },
  writeSkillContent(id: string, content: string): Promise<void> {
    return invoke("write_skill_content", { id, content });
  },
  /** Creates `<skills-dir>/<name>/SKILL.md` with minimal frontmatter and
   *  returns the new skill; the caller opens it in the editor. */
  createSkill(input: {
    tool: AgentTool;
    scope: SkillScope;
    projectPath?: string;
    name: string;
    description: string;
  }): Promise<Skill> {
    return invoke("create_skill", {
      tool: input.tool,
      scope: input.scope,
      projectPath: input.projectPath ?? null,
      name: input.name,
      description: input.description,
    });
  },
};
```

- [ ] **Step 2: Create `src/api/projects.ts`**

Move the project methods verbatim (56–90) including their doc comments, with the `open` import from `@tauri-apps/plugin-dialog`:

```ts
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { DetectedProject, ProjectInfo, Skill } from "../types";

/** Invoke wrappers over the project commands in
 *  src-tauri/src/commands/projects.rs. */
export const projectsApi = {
  // 10 methods, bodies moved verbatim from lib/api.ts lines 56–90,
  // keeping their doc comments: listProjects, listDetectedProjects,
  // refreshDetectedProjects, addProject, removeProject,
  // setProjectPinned, touchProject, listProjectSkillCounts,
  // listProjectSkills, pickProjectFolder
};
```

- [ ] **Step 3: Create `src/api/index.ts`**

```ts
import { projectsApi } from "./projects";
import { skillsApi } from "./skills";

/** Single place the frontend talks to the Rust command layer. */
export const api = { ...skillsApi, ...projectsApi };
```

- [ ] **Step 4: Repoint consumers, delete old file, gate**

In the 7 files listed above change `import { api } from "../lib/api";` → `import { api } from "../api";` then:

```bash
git rm src/lib/api.ts
npx tsc --noEmit && npm test 2>&1 | tail -4
```

Expected: clean; `grep -rn "lib/api" src/` returns nothing.

- [ ] **Step 5: Commit**

```bash
git add src/api src/lib/api.ts src/hooks src/components && git commit -m "refactor(api): split the invoke client into api/skills and api/projects

lib/ mixed the tauri client with pure helpers; the client now lives in
api/ split by domain behind the same composed \"api\" object, so call
sites only change their import path."
```

---

### Task 4: Move `src/lib/` helpers to `src/utils/`

**Files:**
- Rename (all via `git mv`): `src/lib/filterSkills.ts`, `relativeTime.ts`, `projectUsage.ts`, `sidebarLists.ts`, `markdown.ts`, `src/lib/__tests__/` → same names under `src/utils/`
- Delete: `src/lib/` directory (emptied by the moves; Task 3 already removed `api.ts`)

**Interfaces:**
- Produces: same exported function names; relative import depth is unchanged (`../types` from `src/utils/` and `../../types` from `src/utils/__tests__/` resolve exactly as before), so **no** consumer edits are expected.

- [ ] **Step 1: Move and gate**

```bash
mkdir -p src/utils
git mv src/lib/filterSkills.ts src/lib/relativeTime.ts src/lib/projectUsage.ts src/lib/sidebarLists.ts src/lib/markdown.ts src/utils/
git mv src/lib/__tests__ src/utils/__tests__
rmdir src/lib
npx tsc --noEmit && npm test 2>&1 | tail -4
```

Expected: tsc clean; all vitest tests pass from their new location; `ls src/lib` fails (gone).

- [ ] **Step 2: Fix every remaining `lib/` import**

```bash
grep -rn "lib/" src/ --include="*.ts" --include="*.tsx"
```

Expected hits (all rewritten in this step): `src/App.tsx` has `import { filterSkills } from "./lib/filterSkills";` → `./utils/filterSkills`; `src/components/{EditorModal,CreateSkillModal,AddProjectModal,Sidebar,ProjectsModal}.tsx` still import `../lib/markdown`, `../lib/relativeTime`, `../lib/projectUsage`, `../lib/sidebarLists` → change the `lib/` segment to `utils/` (Task 5 will deepen these to `../../utils/` when the files move). Re-run `npx tsc --noEmit` until the grep returns nothing and tsc is clean.

- [ ] **Step 3: Commit**

```bash
git add -A src/lib src/utils src/components src/App.tsx && git commit -m "refactor(utils): move pure helpers out of lib into utils

lib/ now no longer exists; utils/ holds the pure helpers and their
tests, api/ (previous commit) holds the invoke client."
```

---

### Task 5: Regroup `src/components/` by role

**Files:**
- Rename into `layout/`: `Sidebar.tsx`, `Topbar.tsx`
- Rename into `skills/`: `SkillList.tsx`, `SkillCard.tsx`, `SortToggle.tsx`
- Rename into `modals/`: `EditorModal.tsx`, `CreateSkillModal.tsx`, `AddProjectModal.tsx`, `ProjectsModal.tsx`
- Modify: `src/App.tsx` (7 import lines) and the moved files' relative imports

**Interfaces:**
- Produces: same component names and props; only import specifiers change.

- [ ] **Step 1: Move files**

```bash
mkdir -p src/components/layout src/components/skills src/components/modals
git mv src/components/Sidebar.tsx src/components/Topbar.tsx src/components/layout/
git mv src/components/SkillList.tsx src/components/SkillCard.tsx src/components/SortToggle.tsx src/components/skills/
git mv src/components/EditorModal.tsx src/components/CreateSkillModal.tsx src/components/AddProjectModal.tsx src/components/ProjectsModal.tsx src/components/modals/
```

- [ ] **Step 2: Deepen relative imports in moved files (exact final lines)**

Every moved file's `../x` becomes `../../x`; same-directory imports stay. Exact final import lines:

`layout/Sidebar.tsx`: `import { orderToolsByPin, selectVisibleProjects } from "../../utils/sidebarLists";` and `import type { ProjectInfo, ToolEntry, View } from "../../types";`
`layout/Topbar.tsx`: `import type { ToolFolderInfo } from "../../types";`
`skills/SkillCard.tsx` and `skills/SkillList.tsx`: `import type { Skill, ToolEntry } from "../../types";` (SkillList keeps `import { SkillCard } from "./SkillCard";` — moved together)
`modals/EditorModal.tsx`: `import { api } from "../../api";`, `import { renderMarkdown } from "../../utils/markdown";`, `import type { Skill, ToolEntry } from "../../types";`
`modals/CreateSkillModal.tsx`: `import { api } from "../../api";`, `import type { AgentTool, ProjectInfo, Skill, ToolEntry } from "../../types";`
`modals/AddProjectModal.tsx`: `import { api } from "../../api";`, `import { relativeTime } from "../../utils/relativeTime";`, `import { SortToggle } from "../skills/SortToggle";`, `import type { DetectedProject, ProjectInfo } from "../../types";`
`modals/ProjectsModal.tsx`: `import { lastUsed, usageCount } from "../../utils/projectUsage";`, `import { relativeTime } from "../../utils/relativeTime";`, `import { SortToggle } from "../skills/SortToggle";`, `import type { ProjectInfo } from "../../types";`

- [ ] **Step 3: Update `src/App.tsx` imports (exact final lines)**

```ts
import { AddProjectModal } from "./components/modals/AddProjectModal";
import { CreateSkillModal } from "./components/modals/CreateSkillModal";
import { EditorModal } from "./components/modals/EditorModal";
import { ProjectsModal } from "./components/modals/ProjectsModal";
import { Sidebar } from "./components/layout/Sidebar";
import { SkillList } from "./components/skills/SkillList";
import { Topbar } from "./components/layout/Topbar";
```

(`filterSkills` import becomes `./utils/filterSkills` if Task 4 left it as `./lib/filterSkills` — it was already `./lib/filterSkills` → set `./utils/filterSkills`.)

- [ ] **Step 4: Gate**

```bash
npx tsc --noEmit && npm test 2>&1 | tail -4
```

Expected: clean; `grep -rn "components/[A-Z]" src/App.tsx` finds nothing (no flat imports left).

- [ ] **Step 5: Commit**

```bash
git add -A src/components src/App.tsx && git commit -m "refactor(components): group components by role

layout/ (app chrome), skills/ (skill grid), modals/ (the four
dialogs) — the flat folder made every change a guessing game."
```

---

### Task 6: Extract `ui/ModalShell` and `ui/icons`

**Files:**
- Create: `src/components/ui/ModalShell.tsx`, `src/components/ui/icons.tsx`
- Modify: the four files in `src/components/modals/`, `src/components/layout/Sidebar.tsx`

**Interfaces:**
- Produces: `ModalShell({ className?, onClose, children })` and `PinIcon({ size = 12 })` / `CloseIcon({ size = 14 })`.

- [ ] **Step 1: Create `src/components/ui/icons.tsx`**

```tsx
export function PinIcon({ size = 12 }: { size?: number }) {
  return (
    <svg viewBox="0 0 20 20" width={size} height={size} fill="none" stroke="currentColor" strokeWidth="1.6">
      <path d="M7 3h6l-1 5 3 2v2H5v-2l3-2-1-5z" strokeLinejoin="round" />
      <path d="M10 12v5" strokeLinecap="round" />
    </svg>
  );
}

export function CloseIcon({ size = 14 }: { size?: number }) {
  return (
    <svg viewBox="0 0 20 20" width={size} height={size} fill="none" stroke="currentColor" strokeWidth="1.6">
      <path d="M5 5l10 10M15 5 5 15" strokeLinecap="round" />
    </svg>
  );
}
```

- [ ] **Step 2: Create `src/components/ui/ModalShell.tsx`**

```tsx
import { useEffect, type ReactNode } from "react";

interface ModalShellProps {
  /** extra class on the .modal element, e.g. "create-modal" */
  className?: string;
  onClose: () => void;
  children: ReactNode;
}

/** The chrome every modal shares: Escape closes, clicking the overlay
 *  closes, clicks inside the dialog do not. */
export function ModalShell({ className, onClose, children }: ModalShellProps) {
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div
        className={className ? `modal ${className}` : "modal"}
        onClick={(e) => e.stopPropagation()}
      >
        {children}
      </div>
    </div>
  );
}
```

- [ ] **Step 3: Refactor `modals/EditorModal.tsx` (verified exemplar)**

Delete its `useEffect` keydown block (lines 28–34 of the original file) and the outer two wrapper divs; wrap with the shell. Exact before/after:

Before:
```tsx
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose]);
```
After: deleted — `ModalShell` owns it.

Before:
```tsx
  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
```
After:
```tsx
  return (
    <ModalShell onClose={onClose}>
        <div className="modal-header">
```
And the closing `</div></div>` at the end becomes `</ModalShell>`. `useEffect` stays imported only if still used elsewhere in the file (EditorModal's other effect at lines 21–26 stays — keep the import).

Replace the close button's inline SVG (exact block):
```tsx
          <button className="icon-btn square" onClick={onClose} title="close">
            <svg viewBox="0 0 20 20" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="1.6">
              <path d="M5 5l10 10M15 5 5 15" strokeLinecap="round" />
            </svg>
          </button>
```
with:
```tsx
          <button className="icon-btn square" onClick={onClose} title="close">
            <CloseIcon />
          </button>
```
Add: `import { CloseIcon } from "../ui/icons";` and `import { ModalShell } from "../ui/ModalShell";`.

- [ ] **Step 4: Apply the same transformation to the other three modals**

Identical edits in `CreateSkillModal.tsx` (shell gets `className="create-modal"`), `AddProjectModal.tsx` and `ProjectsModal.tsx` (both shells get `className="add-modal"`): delete the per-file keydown `useEffect`, replace overlay/wrapper divs with `<ModalShell className="..." onClose={onClose}>`, swap the header close SVG for `<CloseIcon />` (target the button with `className="icon-btn square"` + `title="close"` — the other inline SVGs in these files, e.g. AddProjectModal's browse-row icon, are NOT close buttons and stay). If a modal's `useEffect` import goes unused after removing the keydown effect, remove it from the react import.

- [ ] **Step 5: Use shared icons in `layout/Sidebar.tsx`**

Delete the local `PinIcon` function (lines 20–27) and add `import { PinIcon } from "../ui/icons";`. No size prop needed — default 12 matches.

- [ ] **Step 6: Gate**

```bash
npx tsc --noEmit && npm test 2>&1 | tail -4
```

Expected: clean, tests pass; `grep -rn "modal-overlay" src/components/modals/` returns nothing (only `ui/ModalShell.tsx` has it).

- [ ] **Step 7: Commit**

```bash
git add src/components && git commit -m "refactor(components): extract shared ModalShell and icons

the escape/overlay/close chrome was duplicated across all four modals
and the pin icon lived inline in the sidebar."
```

---

### Task 7: Update `AGENTS.md`, full verification, dev-app smoke test

**Files:**
- Modify: `AGENTS.md` (Structure section, security note)

**Interfaces:**
- Consumes: final tree from Tasks 1–6.

- [ ] **Step 1: Replace AGENTS.md's structure tree**

Replace the `## Structure` fenced block with:

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

- [ ] **Step 2: Update the security note's file reference**

In the Security note paragraph, change `(`skills_roots` in `commands.rs`)` to `(`skills_roots` / `validate_manifest_at` in `src-tauri/src/commands/mod.rs`)`. Nothing else in the note changes.

- [ ] **Step 3: Full gate suite**

```bash
npx tsc --noEmit && npm test 2>&1 | tail -4
cd src-tauri && cargo check && cargo fmt --check && cargo clippy --all-targets && cargo test 2>&1 | tail -3 && cd ..
```

Expected: everything clean; both test counts equal the Task 1 baselines.

- [ ] **Step 4: Dev-app smoke test**

```bash
npm run tauri dev
```

Manually verify, then quit the app: sidebar renders with tool + project lists; click a tool filter; open a project; open a skill in the editor; Escape closes it; click "new skill", create one, confirm the editor opens on it; Escape closes each remaining modal via overlay click. Expected: no console errors, behavior identical to `main`.

- [ ] **Step 5: Commit and open the PR**

```bash
git add AGENTS.md && git commit -m "docs(agents): describe the new module layout"
git push -u origin refactor/layered-structure
gh pr create --base main --title "refactor: layered structure tidy-up" --body "Implements docs/superpowers/specs/2026-08-21-structure-tidyup-design.md — pure moves plus a shared ModalShell/icons extraction. Rust commands/ split, FE api/ utils/ types/ and component role groups. Zero behavior change; all gates green."
```

Expected: PR opened against `main` for human review.
