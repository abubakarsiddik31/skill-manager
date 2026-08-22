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
- **Built-ins (managed public manifest):** the user's
  `claude-skills-collection` repo hosts a machine-readable
  `collections.json` listing vetted repo-style collections. The app
  fetches it as its built-in catalog — maintainers evaluate repos,
  add entries, and every app install sees the update with no app
  release. A fallback copy is bundled with the app (offline
  first-run), seeded with `anthropics/skills` (20 skills),
  `obra/superpowers` (14), `mattpocock/skills` (36, nested — hence
  any-depth enumeration); all three verified against the trees API on
  2026-08-22. Users can also add any public GitHub repo locally.
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
- README-table parsing of any list, including `claude-skills-collection`'s
  current format — curation happens in the manifest instead (if the
  repo adopts `collections.json`, the README becomes human-facing
  documentation of the same list).

## Architecture

New Rust module tree plus one commands file:

```
src-tauri/src/collections/
  mod.rs        types + orchestration
  github.rs     HTTP wrapper over api.github.com and
                raw.githubusercontent.com (ureq, blocking —
                commands run off the main thread). Sends a User-Agent,
                maps every failure to a typed error. Serves tree
                fetches, blob fetches, and the manifest fetch.
  catalog.rs    Repo-style enumeration via
                GET /repos/{owner}/{repo}/git/trees/{ref}?recursive=1:
                keep blobs named SKILL.md at any depth (skill folder =
                the blob's parent path; a root-level SKILL.md makes the
                whole repo a single skill with path ""). Collections
                are { owner, repo, subpath: Option<String> } — subpath
                scopes enumeration for monorepos. The built-in catalog
                comes from the remote manifest below; a fallback copy
                is compiled in via include_str!.
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

### Built-in catalog manifest

Fetched over HTTPS from

```
https://raw.githubusercontent.com/abubakarsiddik31/
claude-skills-collection/main/collections.json
```

Schema (version-gated; `version: 1`):

```json
{
  "version": 1,
  "collections": [
    { "id": "anthropics-skills", "title": "Anthropic Skills",
      "repo": "anthropics/skills" },
    { "id": "superpowers", "title": "Superpowers",
      "repo": "obra/superpowers" }
  ]
}
```

Fetched when the browse UI loads (cached with the same TTL). On fetch
failure, malformed JSON, or an unsupported `version`, fall back to
cached manifest → bundled copy (`src-tauri/src/collections/
fallback.json`, compiled in with `include_str!`). `list_collections`
merges manifest entries with user-added local collections, deduped by
repo. The manifest only *names* repositories — enumeration and install
are unchanged, so a stale or tampered manifest can at worst advertise
unhelpful repos; it cannot bypass any validation.

Companion change outside this repo: add `collections.json` (seeded
with the three verified repos) to `claude-skills-collection`. Until it
exists there, the bundled fallback is the catalog.

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
  stale badge; only fail when no cache exists. The manifest chain
  degrades the same way (remote → cached → bundled).
- Manifest problems (unreachable, malformed, unsupported version):
  fall back silently to cached/bundled with a subtle notice in the
  collections pane, never a blocking error.
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
  subpath scoping); manifest parsing and the fallback chain (valid,
  malformed, unsupported version → bundled); manifest + user-added
  merge with dedupe; folder-name and relative-path safety;
  symlink skipping; provenance write; collision + overwrite; temp-dir
  cleanup on failure.
- TS (vitest, `src/utils/`): collection search/filter helpers.
- Gates: `npx tsc --noEmit`, `npm test`, `cargo fmt`, `cargo clippy
  --all-targets`, `cargo test`.
