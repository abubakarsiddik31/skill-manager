# Browse & install skills from GitHub collections — design

Date: 2026-08-22
Status: approved (approach chosen from three options: Rust-side GitHub
client over webview-side fetching and a Vercel skills-CLI wrapper).
Revised same day: dropped index-style collections (README-table
parsing) — all sources are deterministic repo-style collections.

## Problem

The app manages skills a user already has, but discovering new ones
means leaving the app: finding a collection on GitHub, cloning or
copy-pasting a skill folder into the right agent directory by hand.
Collections exist and are thriving — `claude-skills-collection` is a
227-entry index, `anthropics/skills` and `obra/superpowers` hold real
skill folders — but installing from any of them is manual and
error-prone (wrong folder, wrong agent dir, no description metadata).

We want: browse skills from curated collections inside the app, pick a
target agent, and install in one click.

## Decisions (from brainstorming)

- **Source model:** GitHub-native, repo-style collections only. Every
  source is a public repo whose skills are folders containing a
  `SKILL.md` (the Agent Skills open standard,
  `agentskills/agentskills`), enumerated deterministically via the
  GitHub git-trees API. No README/index parsing — index lists like
  `claude-skills-collection` are non-deterministic to parse and were
  dropped. No skills.sh / Vercel integration.
- **Built-ins:** `anthropics/skills` (20 skills, `skills/<name>/`),
  `obra/superpowers` (14, `skills/<name>/`), `mattpocock/skills`
  (36, `skills/engineering/<name>/` — nested, hence any-depth
  enumeration). All three verified against the trees API on
  2026-08-22. Users can add any other public GitHub repo.
- **After install:** record provenance (`.collection-source.json` in the
  skill folder) so an update feature can be built later without
  name-matching. No update UI in v1.
- **Networking:** entirely in Rust behind new commands. The webview
  gains no new capabilities; CSP and `capabilities/default.json` stay
  untouched.

## Goals

- Browse built-in and user-added GitHub collections in a modal, with
  search over name/description.
- Install a skill into any managed agent folder (user or project scope)
  through the same picker pattern `CreateSkillModal` uses.
- Installs pass the existing managed-path security model: target must
  be in `skills_roots()`, folder name validated, collisions explicit.
- Browse results cached so the unauthenticated GitHub API limit
  (~60 req/hr per IP) is not a practical constraint.

## Non-goals (v1)

- Update checking / re-install diffing (provenance is recorded only).
- skills.sh, marketplace sites, or any non-GitHub source.
- GitHub authentication (token) support.
- Bulk "install whole collection".
- Symlink downloads (tree entries with mode `120000` are skipped).
- Index/README-table sources of any kind (including
  `claude-skills-collection`) — non-deterministic to parse; could
  return later as a structured manifest if that repo ever ships one.

## Architecture

New Rust module tree plus one commands file:

```
src-tauri/src/collections/
  mod.rs        types + orchestration
  github.rs     HTTP wrapper over api.github.com (ureq, blocking —
                commands run off the main thread). Sends a User-Agent,
                maps every failure to a typed error.
  catalog.rs    Repo-style enumeration via
                GET /repos/{owner}/{repo}/git/trees/{ref}?recursive=1:
                keep blobs named SKILL.md at any depth (skill folder =
                the blob's parent path; a root-level SKILL.md makes the
                whole repo a single skill with path ""). Collections
                are { owner, repo, subpath: Option<String> } — subpath
                scopes enumeration for monorepos. Built-in constants
                live here.
  install.rs    download a skill folder's blobs by SHA, write into a
                managed root, write provenance.
  store.rs      user-added collections persisted to collections.json
                in app_config_dir, same pattern as projects.json.
src-tauri/src/commands/collections.rs
  list_collections, browse_collection, refresh_collection,
  add_collection, remove_collection, install_skill,
  fetch_skill_manifest
```

Tree fetches are cached **per (owner, repo, ref)**, so browsing and
installing from the same repo share one enumeration.

Built-in collection constants live in `catalog.rs`; the default branch
is resolved once per repo via `GET /repos/{owner}/{repo}` and cached.

## Data model

```rust
// catalog
struct CollectionInfo { id, title, kind, source_url, builtin, skill_count: Option<u64> }
struct RemoteSkill   { name, description: Option<String>, owner, repo, path, branch }

// install
struct InstallRequest { skill: RemoteSkill, tool: AgentTool,
                        scope: SkillScope, project_path: Option<PathBuf>,
                        overwrite: bool }

// .collection-source.json, written inside the installed folder
struct Provenance { owner, repo, path, branch, tree_sha, installed_at, collection_id }
```

Descriptions: the trees API returns paths only, so entries start with
`description: None` and the grid fills them lazily —
`fetch_skill_manifest` fetches the SKILL.md blob (already keyed by SHA
in the cached tree) when a card becomes visible, with results cached by
blob SHA. `tree_sha` records the enumerated tree object's SHA — the
artifact we actually have; per-folder commit SHAs would cost an extra
API call per skill and buy nothing for v1.

`collections-cache.json` (app_config_dir): per collection id —
`{ fetched_at, skills: Vec<RemoteSkill> }`; per repo — resolved branch
and tree. TTL 24 h, plus a manual refresh button that bypasses it.

Frontend types mirror these in `src/types/collection.ts`, exported
through the `types/index.ts` barrel.

## Install flow (security-critical)

1. Resolve target root exactly like `create_skill`
   (src-tauri/src/commands/create_skill.rs:117): user scope →
   `adapter.skills_dir()`; project scope → `project_path` must be a
   tracked project, root = `<project>/<project_subpath>`.
2. Folder name passes `validate_skill_folder_name`; name source is the
   remote folder name (SKILL.md frontmatter `name` may differ and is
   not used for the local folder).
3. Collision: target folder exists → typed error. With
   `overwrite: true` the existing folder is removed through the same
   validated delete path `delete_skill` uses, then install proceeds.
4. Download blobs for the remote folder (tree is cached; blob SHAs are
   in it). Every relative path from the tree must be plain: no `..`,
   no absolute components; symlink entries (mode `120000`) are skipped
   and reported.
5. Write to `<root>/<name>.tmp-<random>/`, then rename into
   `<root>/<name>/`; on any failure remove the temp dir (mirrors
   create_skill's cleanup).
6. Write `.collection-source.json` next to SKILL.md.
7. Return the new skill via `find_skill_by_manifest` and clear the
   project's cached skill count, as `create_skill` does.

The webview performs no network I/O and no direct file writes; every
mutation stays behind the managed-path validation.

## Frontend

- `src/api/collections.ts` — invoke wrappers, re-exported from
  `src/api/index.ts`.
- `src/hooks/useCollections.ts` — load collection list, browse the
  active collection, race-guarded refresh (pattern of
  `useGlobalSkills`).
- `src/components/modals/BrowseModal.tsx` — wraps `ModalShell`. Left
  pane: collections with add (URL input validated as `owner/repo`) and
  remove (user-added only). Right pane: searchable card grid (name,
  description — filled lazily, see Data model — and source repo).
  "Add" on a card reveals the tool + scope
  picker reused from `CreateSkillModal`; install → success state →
  grid stays open for further installs.
- Topbar gains a **Browse** button next to "new skill".

## Error handling

- Offline or GitHub unreachable: serve the stale cache with a visible
  stale badge; only fail when no cache exists.
- HTTP 429 / rate limit: friendly message with retry hint.
- `add_collection`: validate `owner/repo` shape, then probe-fetch to
  confirm reachability; report zero-skill repos as a warning, not an
  error.
- Install failures mid-download leave no partial folder (temp-dir +
  rename).

## Testing

- Rust (fixture-backed `GithubHttp` trait impl; no live network in
  tests): tree → skills filtering (any depth, nested layouts like
  `skills/engineering/<name>/`, root-SKILL.md single-skill repos,
  subpath scoping); folder-name and relative-path safety;
  symlink skipping; provenance write; collision + overwrite; temp-dir
  cleanup on failure.
- TS (vitest, `src/utils/`): collection search/filter helpers.
- Gates: `npx tsc --noEmit`, `npm test`, `cargo fmt`, `cargo clippy
  --all-targets`, `cargo test`.
