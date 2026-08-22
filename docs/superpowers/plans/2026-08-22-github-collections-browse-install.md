# GitHub Collections Browse & Install — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Browse skills from GitHub collections (built-in public manifest + user-added repos) and install them into any managed agent folder, recording provenance for future updates.

**Architecture:** All networking lives in a new Rust module `src-tauri/src/collections/` behind new Tauri commands — the webview keeps its closed CSP and gains no capabilities. Collections are repo-style only (`SKILL.md` folders enumerated via the GitHub git-trees API, any depth). The built-in catalog is a remote JSON manifest (`claude-skills-collection/collections.json`) with a bundled fallback; installs reuse the `create_skill` security model (managed roots, folder-name validation, collision checks) and write a `.collection-source.json` provenance file.

**Tech Stack:** Rust (tauri 2, ureq 2, base64, serde), React + TypeScript, vitest.

**Spec:** `docs/superpowers/specs/2026-08-22-github-collections-browse-install-design.md`

## Global Constraints

- Gates before any commit: `npx tsc --noEmit` clean, `npm test` green, `cargo fmt` leaves no diff, `cargo clippy --all-targets` warning-free, `cargo test` green (run from `src-tauri/` for cargo).
- No `Co-Authored-By` trailer. Commit messages: imperative, lowercase start (`feat(collections): ...`), body explains why when non-obvious.
- Security invariants (from AGENTS.md): the webview is untrusted — no new webview capabilities, CSP untouched; every file write lands only inside `skills_roots()`; remote folder names pass `validate_skill_folder_name`; remote relative paths are checked `..`-free before writing; symlink/submodule tree entries are never materialized.
- No live network in tests — all HTTP goes through the `GithubHttp` trait with fixture-backed fakes.
- TypeScript imports stay relative (no path aliases). Frontend types are camelCase on the wire (serde `rename_all = "camelCase"` everywhere new).
- Version stays 0.3.1 — releasing is a separate concern.

---

### Task 1: GitHub HTTP layer (`github.rs`)

**Files:**
- Modify: `src-tauri/Cargo.toml` (deps)
- Create: `src-tauri/src/collections/mod.rs`
- Create: `src-tauri/src/collections/github.rs`
- Modify: `src-tauri/src/lib.rs:4` (add `mod collections;`)

**Interfaces:**
- Consumes: nothing new.
- Produces: `GithubHttp` trait (`get_text`, `fetch_tree`, `fetch_blob`, `fetch_default_branch`), `UreqGithubHttp`, `TreeResponse { sha, tree: Vec<TreeEntry>, truncated }`, `TreeEntry { path, mode, sha, kind }`, pure parsers `parse_tree`, `parse_blob`, `parse_default_branch`, `api_url`. Test-only: `FakeGithubHttp` in `github::testutil`.

- [ ] **Step 1: Add dependencies**

In `src-tauri/Cargo.toml`, extend `[dependencies]`:

```toml
ureq = "2"
base64 = "0.22"
```

- [ ] **Step 2: Create the module skeleton**

Create `src-tauri/src/collections/mod.rs`:

```rust
pub mod github;

pub use github::{GithubHttp, TreeEntry, TreeResponse, UreqGithubHttp};
```

Add to `src-tauri/src/lib.rs` after `mod commands;` (line 1 area, keeping alpha order):

```rust
mod collections;
```

(`detect`, `projects`, `skills` follow alphabetically — insert `mod collections;` as the first entry.)

- [ ] **Step 3: Write the failing tests for the parsers**

Create `src-tauri/src/collections/github.rs` with only the test module and type stubs so it compiles, then run tests to see them fail:

```rust
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests {
    use super::*;

    const TREE_JSON: &str = r#"{
        "sha": "abc123",
        "truncated": false,
        "tree": [
            {"path": "skills/brand-guidelines/SKILL.md", "mode": "100644", "sha": "b1", "type": "blob"},
            {"path": "skills/brand-guidelines", "mode": "040000", "sha": "t1", "type": "tree"},
            {"path": "README.md", "mode": "100644", "sha": "b2", "type": "blob"},
            {"path": "linked", "mode": "120000", "sha": "b3", "type": "blob"}
        ]
    }"#;

    #[test]
    fn parse_tree_keeps_all_entries_with_flags() {
        let tree = parse_tree(TREE_JSON).unwrap();
        assert_eq!(tree.sha, "abc123");
        assert!(!tree.truncated);
        assert_eq!(tree.tree.len(), 4);
        assert_eq!(tree.tree[0].kind, "blob");
        assert_eq!(tree.tree[1].kind, "tree");
        assert_eq!(tree.tree[3].mode, "120000");
    }

    #[test]
    fn parse_tree_defaults_truncated_to_false() {
        let tree = parse_tree(r#"{"sha": "x", "tree": []}"#).unwrap();
        assert!(!tree.truncated);
    }

    #[test]
    fn parse_tree_rejects_malformed_json() {
        assert!(parse_tree("{ not json").is_err());
    }

    #[test]
    fn parse_blob_decodes_base64_with_newlines() {
        // GitHub wraps blob content at 60 columns with literal newlines.
        let json = r#"{"content": "aGVs\nbG8=\n", "encoding": "base64"}"#;
        assert_eq!(parse_blob(json).unwrap(), b"hello");
    }

    #[test]
    fn parse_blob_rejects_unknown_encoding() {
        assert!(parse_blob(r#"{"content": "x", "encoding": "utf-16"}"#).is_err());
    }

    #[test]
    fn parse_default_branch_reads_field() {
        let json = r#"{"default_branch": "main", "full_name": "a/b"}"#;
        assert_eq!(parse_default_branch(json).unwrap(), "main");
    }
}
```

Run: `cd src-tauri && cargo test collections::github`
Expected: FAIL — `parse_tree` etc. not defined.

- [ ] **Step 4: Implement the types, parsers, trait, and ureq impl**

Replace `src-tauri/src/collections/github.rs` with the full implementation (keep the test module from Step 3 verbatim):

```rust
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// GitHub requires a User-Agent on every request; identify the app.
pub const USER_AGENT: &str = "skill-manager (https://github.com/abubakarsiddik31/skill-manager)";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeEntry {
    pub path: String,
    /// Git mode: 100644/100755 files, 120000 symlinks, 160000 submodules.
    pub mode: String,
    pub sha: String,
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeResponse {
    pub sha: String,
    pub tree: Vec<TreeEntry>,
    /// GitHub truncates very large trees; such repos cannot be browsed.
    #[serde(default)]
    pub truncated: bool,
}

pub fn api_url(path: &str) -> String {
    format!("https://api.github.com{path}")
}

pub fn parse_tree(json: &str) -> Result<TreeResponse, String> {
    serde_json::from_str(json).map_err(|e| format!("cannot parse tree response: {e}"))
}

/// Blob responses carry base64 content wrapped at 60 columns — strip the
/// newlines before decoding, and refuse any non-base64 encoding.
pub fn parse_blob(json: &str) -> Result<Vec<u8>, String> {
    #[derive(Deserialize)]
    struct Raw {
        content: String,
        encoding: String,
    }
    let raw: Raw =
        serde_json::from_str(json).map_err(|e| format!("cannot parse blob response: {e}"))?;
    if raw.encoding != "base64" {
        return Err(format!("unsupported blob encoding '{}'", raw.encoding));
    }
    let compact: String = raw.content.chars().filter(|c| *c != '\n' && *c != '\r').collect();
    BASE64
        .decode(compact.as_bytes())
        .map_err(|e| format!("cannot decode blob content: {e}"))
}

pub fn parse_default_branch(json: &str) -> Result<String, String> {
    #[derive(Deserialize)]
    struct Raw {
        default_branch: String,
    }
    let raw: Raw =
        serde_json::from_str(json).map_err(|e| format!("cannot parse repo response: {e}"))?;
    if raw.default_branch.is_empty() {
        return Err("repository has no default branch".into());
    }
    Ok(raw.default_branch)
}

/// The one seam every GitHub call goes through. Tests provide fixture
/// implementations; production uses `UreqGithubHttp`.
pub trait GithubHttp: Send + Sync {
    fn get_text(&self, url: &str) -> Result<String, String>;

    fn fetch_tree(
        &self,
        owner: &str,
        repo: &str,
        reference: &str,
    ) -> Result<TreeResponse, String> {
        let url = api_url(&format!("/repos/{owner}/{repo}/git/trees/{reference}?recursive=1"));
        self.get_text(&url).and_then(|t| parse_tree(&t))
    }

    fn fetch_blob(&self, owner: &str, repo: &str, sha: &str) -> Result<Vec<u8>, String> {
        let url = api_url(&format!("/repos/{owner}/{repo}/git/blobs/{sha}"));
        self.get_text(&url).and_then(|t| parse_blob(&t))
    }

    fn fetch_default_branch(&self, owner: &str, repo: &str) -> Result<String, String> {
        let url = api_url(&format!("/repos/{owner}/{repo}"));
        self.get_text(&url).and_then(|t| parse_default_branch(&t))
    }
}

pub struct UreqGithubHttp;

impl GithubHttp for UreqGithubHttp {
    fn get_text(&self, url: &str) -> Result<String, String> {
        match ureq::get(url)
            .set("User-Agent", USER_AGENT)
            .timeout(Duration::from_secs(30))
            .call()
        {
            Ok(response) => response
                .into_string()
                .map_err(|e| format!("cannot read response: {e}")),
            Err(ureq::Error::Status(404, _)) => Err(format!("not found: {url}")),
            Err(ureq::Error::Status(403, _)) | Err(ureq::Error::Status(429, _)) => {
                Err("GitHub rate limit reached — wait a few minutes and refresh".into())
            }
            Err(ureq::Error::Status(code, _)) => Err(format!("GitHub returned HTTP {code}")),
            Err(e) => Err(format!("cannot reach GitHub: {e}")),
        }
    }
}

/// Fixture-backed HTTP for tests: map URL suffixes to canned bodies.
#[cfg(test)]
pub(crate) mod testutil {
    use super::*;
    use std::collections::HashMap;

    pub struct FakeGithubHttp {
        pub bodies: HashMap<String, String>,
    }

    impl FakeGithubHttp {
        pub fn new() -> Self {
            Self {
                bodies: HashMap::new(),
            }
        }

        pub fn mount(&mut self, url_suffix: &str, body: &str) {
            self.bodies.insert(url_suffix.to_string(), body.to_string());
        }
    }

    impl Default for FakeGithubHttp {
        fn default() -> Self {
            Self::new()
        }
    }

    impl GithubHttp for FakeGithubHttp {
        fn get_text(&self, url: &str) -> Result<String, String> {
            self.bodies
                .iter()
                .find(|(suffix, _)| url.ends_with(suffix.as_str()))
                .map(|(_, body)| body.clone())
                .ok_or_else(|| format!("no fixture for {url}"))
        }
    }
}
```

- [ ] **Step 5: Run the tests**

Run: `cd src-tauri && cargo test collections::github`
Expected: PASS (6 tests).

- [ ] **Step 6: Gate and commit**

Run: `cd src-tauri && cargo fmt && cargo clippy --all-targets && cargo test`
Expected: all clean/green.

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/collections src-tauri/src/lib.rs
git commit -m "feat(collections): add github http layer with tree/blob parsing"
```

---

### Task 2: Tree → skills enumeration (`catalog.rs`)

**Files:**
- Create: `src-tauri/src/collections/catalog.rs`
- Modify: `src-tauri/src/collections/mod.rs`

**Interfaces:**
- Consumes: `GithubHttp`, `TreeResponse`, `TreeEntry` (Task 1).
- Produces: `RemoteSkill { name, description: Option<String>, owner, repo, path, branch }` (in `mod.rs`), `pub fn skills_from_tree(owner: &str, repo: &str, branch: &str, tree: &TreeResponse, subpath: Option<&str>) -> Vec<RemoteSkill>`, `pub fn enumerate_repo_skills(http: &dyn GithubHttp, owner: &str, repo: &str, subpath: Option<&str>) -> Result<(String, TreeResponse, Vec<RemoteSkill>), String>` returning `(branch, tree, skills)`.

- [ ] **Step 1: Add `RemoteSkill` to `mod.rs`**

In `src-tauri/src/collections/mod.rs` add (below the module declarations):

```rust
pub mod catalog;

pub use catalog::{enumerate_repo_skills, skills_from_tree};

use serde::{Deserialize, Serialize};

/// One installable skill in a remote collection. `path` is the folder
/// containing its SKILL.md ("" when the repo root is the skill);
/// `description` is filled lazily from the SKILL.md blob.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteSkill {
    pub name: String,
    pub description: Option<String>,
    pub owner: String,
    pub repo: String,
    pub path: String,
    pub branch: String,
}
```

- [ ] **Step 2: Write the failing tests**

Create `src-tauri/src/collections/catalog.rs`:

```rust
use super::github::TreeResponse;
use super::RemoteSkill;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::github::testutil::FakeGithubHttp;
    use crate::collections::github::parse_tree;

    const REPO_TREE: &str = r#"{
        "sha": "tree-sha",
        "truncated": false,
        "tree": [
            {"path": "README.md", "mode": "100644", "sha": "b0", "type": "blob"},
            {"path": "skills/brainstorming/SKILL.md", "mode": "100644", "sha": "b1", "type": "blob"},
            {"path": "skills/brainstorming/references/x.md", "mode": "100644", "sha": "b2", "type": "blob"},
            {"path": "skills/engineering/code-review/SKILL.md", "mode": "100644", "sha": "b3", "type": "blob"},
            {"path": "skills/engineering/code-review/scripts/run.py", "mode": "100755", "sha": "b4", "type": "blob"},
            {"path": "skills/engineering", "mode": "040000", "sha": "t1", "type": "tree"},
            {"path": "docs/skill.md", "mode": "100644", "sha": "b5", "type": "blob"}
        ]
    }"#;

    fn tree() -> TreeResponse {
        parse_tree(REPO_TREE).unwrap()
    }

    #[test]
    fn finds_skill_folders_at_any_depth() {
        let skills = skills_from_tree("obra", "superpowers", "main", &tree(), None);
        let names: Vec<&str> = skills.iter().map(|s| s.name.as_str()).collect();
        // nested `skills/engineering/code-review` and flat
        // `skills/brainstorming` both count; `docs/skill.md` (wrong file
        // name) and plain files do not
        assert_eq!(names, ["code-review", "brainstorming"]);
    }

    #[test]
    fn skills_carry_full_source_coordinates() {
        let skills = skills_from_tree("obra", "superpowers", "main", &tree(), None);
        let review = skills.iter().find(|s| s.name == "code-review").unwrap();
        assert_eq!(review.owner, "obra");
        assert_eq!(review.repo, "superpowers");
        assert_eq!(review.path, "skills/engineering/code-review");
        assert_eq!(review.branch, "main");
        assert!(review.description.is_none());
    }

    #[test]
    fn subpath_scopes_enumeration() {
        let skills = skills_from_tree("obra", "superpowers", "main", &tree(), Some("skills/engineering"));
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "code-review");
        // a trailing slash on the subpath must not break matching
        let skills = skills_from_tree("obra", "superpowers", "main", &tree(), Some("skills/engineering/"));
        assert_eq!(skills.len(), 1);
    }

    #[test]
    fn root_skill_md_makes_the_repo_a_single_skill() {
        let single = r#"{"sha": "x", "tree": [
            {"path": "SKILL.md", "mode": "100644", "sha": "b1", "type": "blob"},
            {"path": "scripts/run.py", "mode": "100644", "sha": "b2", "type": "blob"}
        ]}"#;
        let skills = skills_from_tree("me", "one-skill-repo", "main", &parse_tree(single).unwrap(), None);
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].path, "");
        assert_eq!(skills[0].name, "one-skill-repo");
    }

    #[test]
    fn enumerate_resolves_branch_then_tree() {
        let mut http = FakeGithubHttp::new();
        http.mount("/repos/obra/superpowers", r#"{"default_branch": "trunk"}"#);
        http.mount("recursive=1", REPO_TREE);
        let (branch, tree, skills) =
            enumerate_repo_skills(&http, "obra", "superpowers", None).unwrap();
        assert_eq!(branch, "trunk");
        assert_eq!(tree.sha, "tree-sha");
        assert_eq!(skills.len(), 2);
    }

    #[test]
    fn enumerate_rejects_truncated_trees() {
        let mut http = FakeGithubHttp::new();
        http.mount("/repos/a/b", r#"{"default_branch": "main"}"#);
        http.mount("recursive=1", r#"{"sha": "x", "truncated": true, "tree": []}"#);
        let err = enumerate_repo_skills(&http, "a", "b", None).unwrap_err();
        assert!(err.contains("too large"), "unexpected error: {err}");
    }
}
```

Run: `cd src-tauri && cargo test collections::catalog`
Expected: FAIL — functions not defined.

- [ ] **Step 3: Implement**

Add to `src-tauri/src/collections/catalog.rs` (above the test module):

```rust
/// Derive the installable skills from a recursive git tree: every blob
/// named `SKILL.md` marks its parent folder as a skill, at any depth
/// (anthropic-style `skills/<name>/` and nested
/// `skills/engineering/<name>/` layouts both work). Optionally scoped
/// to a subpath for monorepos.
pub fn skills_from_tree(
    owner: &str,
    repo: &str,
    branch: &str,
    tree: &TreeResponse,
    subpath: Option<&str>,
) -> Vec<RemoteSkill> {
    let prefix = subpath
        .map(|p| p.trim_end_matches('/'))
        .filter(|p| !p.is_empty())
        .map(str::to_string);

    let mut skills: Vec<RemoteSkill> = Vec::new();
    for entry in &tree.tree {
        if entry.kind != "blob" {
            continue;
        }
        let folder = match entry.path.strip_suffix("/SKILL.md") {
            Some(f) => f.to_string(),
            None if entry.path == "SKILL.md" => String::new(),
            None => continue,
        };
        if let Some(p) = &prefix {
            let in_scope = folder == *p || folder.starts_with(&format!("{p}/"));
            if !in_scope {
                continue;
            }
        }
        if skills.iter().any(|s| s.path == folder) {
            continue; // one entry per folder, whatever the tree repeats
        }
        let name = if folder.is_empty() {
            repo.to_string()
        } else {
            folder.rsplit('/').next().unwrap_or(repo).to_string()
        };
        skills.push(RemoteSkill {
            name,
            description: None,
            owner: owner.to_string(),
            repo: repo.to_string(),
            path: folder,
            branch: branch.to_string(),
        });
    }
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    skills
}

/// Resolve the repo's default branch, fetch its recursive tree, and
/// derive the skills. Returns `(branch, tree, skills)` — the tree is
/// needed again at install time for blob SHAs.
pub fn enumerate_repo_skills(
    http: &dyn GithubHttp,
    owner: &str,
    repo: &str,
    subpath: Option<&str>,
) -> Result<(String, TreeResponse, Vec<RemoteSkill>), String> {
    let branch = http.fetch_default_branch(owner, repo)?;
    let tree = http.fetch_tree(owner, repo, &branch)?;
    if tree.truncated {
        return Err(format!("repository '{owner}/{repo}' is too large to enumerate"));
    }
    let skills = skills_from_tree(owner, repo, &branch, &tree, subpath);
    Ok((branch, tree, skills))
}

use super::github::GithubHttp;
```

(Move the `use super::github::GithubHttp;` line to the top of the file with the other imports rather than the bottom — shown last here only to keep the diff minimal.)

- [ ] **Step 4: Run the tests**

Run: `cd src-tauri && cargo test collections::catalog`
Expected: PASS (6 tests).

- [ ] **Step 5: Gate and commit**

Run: `cd src-tauri && cargo fmt && cargo clippy --all-targets && cargo test`

```bash
git add src-tauri/src/collections
git commit -m "feat(collections): enumerate skills from a repo tree"
```

---

### Task 3: Public catalog manifest + bundled fallback (`manifest.rs`)

**Files:**
- Create: `src-tauri/src/collections/manifest.rs`
- Create: `src-tauri/src/collections/fallback.json`
- Modify: `src-tauri/src/collections/mod.rs`

**Interfaces:**
- Consumes: `GithubHttp` (Task 1); `split_repo` (added to `mod.rs` this task); `UserCollection`, `CollectionsCache`, `ManifestCache` (Task 4 — this task only references them in `merge_collections`/`load_catalog` signatures, so implement those types as stubs here and fully in Task 4. **Simplification:** define `UserCollection`, `ManifestCache`, and `CollectionsCache` in `store.rs` in Task 4; to keep Task 3 compiling, implement `load_catalog` and `merge_collections` in Task 4 as well — Task 3 delivers only `parse_manifest`, `CATALOG_URL`, `BUNDLED_MANIFEST`, and `fallback.json`.)
- Produces: `ManifestCollection { id, title, repo, subpath }`, `CatalogSource { Manifest | Cached | Bundled }`, `pub fn split_repo(repo: &str) -> Option<(String, String)>` (in `mod.rs`).

- [ ] **Step 1: Add `split_repo` to `mod.rs`**

Append to `src-tauri/src/collections/mod.rs`:

```rust
/// Split an `"owner/repo"` slug into its parts, accepting exactly that
/// shape with GitHub-legal characters. Used to validate both manifest
/// entries and user input.
pub fn split_repo(repo: &str) -> Option<(String, String)> {
    let (owner, name) = repo.split_once('/')?;
    let valid = |s: &str| {
        !s.is_empty()
            && !s.starts_with('.')
            && !s.ends_with('.')
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
    };
    if name.contains('/') || !valid(owner) || !valid(name) {
        return None;
    }
    Some((owner.to_string(), name.to_string()))
}
```

- [ ] **Step 2: Write the failing tests**

Create `src-tauri/src/collections/manifest.rs`:

```rust
use super::ManifestCollection;

pub const CATALOG_URL: &str = "https://raw.githubusercontent.com/abubakarsiddik31/claude-skills-collection/main/collections.json";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_version_one_manifest() {
        let json = r#"{"version": 1, "collections": [
            {"id": "anthropics-skills", "title": "Anthropic Skills", "repo": "anthropics/skills"},
            {"id": "superpowers", "title": "Superpowers", "repo": "obra/superpowers", "subpath": "skills"}
        ]}"#;
        let cols = parse_manifest(json).unwrap();
        assert_eq!(cols.len(), 2);
        assert_eq!(cols[0].id, "anthropics-skills");
        assert_eq!(cols[1].subpath.as_deref(), Some("skills"));
    }

    #[test]
    fn rejects_unsupported_versions() {
        let json = r#"{"version": 2, "collections": []}"#;
        let err = parse_manifest(json).unwrap_err();
        assert!(err.contains("not supported"), "unexpected error: {err}");
    }

    #[test]
    fn rejects_malformed_repos() {
        let json = r#"{"version": 1, "collections": [
            {"id": "x", "title": "X", "repo": "not-a-slug"}
        ]}"#;
        assert!(parse_manifest(json).is_err());
    }

    #[test]
    fn the_bundled_fallback_is_a_valid_manifest() {
        let cols = parse_manifest(BUNDLED_MANIFEST).unwrap();
        assert!(cols.len() >= 3, "fallback should seed the three verified repos");
        assert!(cols.iter().any(|c| c.repo == "anthropics/skills"));
        assert!(cols.iter().any(|c| c.repo == "obra/superpowers"));
        assert!(cols.iter().any(|c| c.repo == "mattpocock/skills"));
    }
}
```

Run: `cd src-tauri && cargo test collections::manifest`
Expected: FAIL.

- [ ] **Step 3: Create `fallback.json` and implement**

Create `src-tauri/src/collections/fallback.json`:

```json
{
  "version": 1,
  "collections": [
    { "id": "anthropics-skills", "title": "Anthropic Skills", "repo": "anthropics/skills" },
    { "id": "superpowers", "title": "Superpowers", "repo": "obra/superpowers" },
    { "id": "matt-skills", "title": "Matt's Skills", "repo": "mattpocock/skills" }
  ]
}
```

Implement in `src-tauri/src/collections/manifest.rs` (above the tests):

```rust
use serde::{Deserialize, Serialize};

/// The manifest compiled into the binary so a fresh install can browse
/// before it ever reaches GitHub (and when GitHub is unreachable).
pub const BUNDLED_MANIFEST: &str = include_str!("fallback.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestCollection {
    pub id: String,
    pub title: String,
    pub repo: String,
    #[serde(default)]
    pub subpath: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CatalogSource {
    Manifest,
    Cached,
    Bundled,
}

/// Parse and validate a catalog manifest. Only `version: 1` is
/// understood; every `repo` must be a legal `owner/repo` slug.
pub fn parse_manifest(json: &str) -> Result<Vec<ManifestCollection>, String> {
    #[derive(Deserialize)]
    struct Raw {
        version: u32,
        collections: Vec<ManifestCollection>,
    }
    let raw: Raw =
        serde_json::from_str(json).map_err(|e| format!("catalog manifest is malformed: {e}"))?;
    if raw.version != 1 {
        return Err(format!("catalog manifest version {} is not supported", raw.version));
    }
    for collection in &raw.collections {
        if super::split_repo(&collection.repo).is_none() {
            return Err(format!(
                "catalog entry '{}' has an invalid repo '{}'",
                collection.id, collection.repo
            ));
        }
    }
    Ok(raw.collections)
}
```

Add to `src-tauri/src/collections/mod.rs`:

```rust
pub mod manifest;

pub use manifest::{parse_manifest, CatalogSource, ManifestCollection, BUNDLED_MANIFEST, CATALOG_URL};
```

- [ ] **Step 4: Run the tests**

Run: `cd src-tauri && cargo test collections::manifest`
Expected: PASS (4 tests).

- [ ] **Step 5: Gate and commit**

Run: `cd src-tauri && cargo fmt && cargo clippy --all-targets && cargo test`

```bash
git add src-tauri/src/collections
git commit -m "feat(collections): parse the public catalog manifest with bundled fallback"
```

---

### Task 4: Local persistence — user collections + browse cache (`store.rs`) and catalog chain

**Files:**
- Create: `src-tauri/src/collections/store.rs`
- Modify: `src-tauri/src/collections/manifest.rs` (add `load_catalog`, `merge_collections`)
- Modify: `src-tauri/src/collections/mod.rs`

**Interfaces:**
- Consumes: `TreeResponse` (Task 1), `ManifestCollection`/`CatalogSource`/`parse_manifest`/`BUNDLED_MANIFEST` (Task 3), `GithubHttp` (Task 1).
- Produces: `UserCollection { id, title, owner, repo, subpath, added_at }`, `RepoCache { fetched_at, branch, tree }`, `ManifestCache { fetched_at, raw }`, `CollectionsCache { manifest: Option<ManifestCache>, repos: HashMap<String, RepoCache> }`, `CollectionInfo { id, title, owner, repo, subpath, builtin, skill_count }` (in `mod.rs`), `load_catalog(http, cache, now) -> (Vec<ManifestCollection>, CatalogSource)`, `merge_collections(manifest, user) -> Vec<CollectionInfo>`, plus `now_secs`, `cache_fresh`, load/save/add/remove store functions (all take a `&Path` config dir for testability).

- [ ] **Step 1: Add `CollectionInfo` to `mod.rs`**

```rust
pub mod store;

pub use store::{
    add_user_collection, cache_fresh, load_cache, load_user_collections, now_secs,
    remove_user_collection, save_cache, save_user_collections, CollectionsCache, ManifestCache,
    RepoCache, UserCollection,
};
```

And the type (next to `RemoteSkill`):

```rust
/// One browsable collection, regardless of where it came from.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionInfo {
    pub id: String,
    pub title: String,
    pub owner: String,
    pub repo: String,
    pub subpath: Option<String>,
    pub builtin: bool,
    pub skill_count: Option<u64>,
}
```

- [ ] **Step 2: Write the failing tests for `store.rs`**

Create `src-tauri/src/collections/store.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir()
            .join(format!("skill-manager-store-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn user_collections_round_trip() {
        let dir = tmp_dir("user");
        assert!(load_user_collections(&dir).is_empty());

        let added = add_user_collection(&dir, "anthropics/skills", None).unwrap();
        assert_eq!(added.id, "anthropics/skills");
        assert_eq!(added.title, "skills"); // repo name is the default title
        assert_eq!(added.owner, "anthropics");

        let stored = load_user_collections(&dir);
        assert_eq!(stored.len(), 1);

        let dup = add_user_collection(&dir, "anthropics/skills", None).unwrap_err();
        assert!(dup.contains("already"), "unexpected error: {dup}");

        remove_user_collection(&dir, "anthropics/skills").unwrap();
        assert!(load_user_collections(&dir).is_empty());
        assert!(remove_user_collection(&dir, "anthropics/skills").is_err());

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn add_rejects_bad_slugs() {
        let dir = tmp_dir("slug");
        for bad in ["nodir", "a/b/c", "../escape", "a/","/b", "a b/c", ".a/b"] {
            assert!(add_user_collection(&dir, bad, None).is_err(), "{bad:?} should be rejected");
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn cache_round_trip_and_freshness() {
        let dir = tmp_dir("cache");
        assert!(cache_fresh(1_000, 1_000 + 10));
        assert!(!cache_fresh(1_000, 1_000 + CACHE_TTL_SECS + 1));
        // clock skew (future fetch time) must read as fresh, not negative
        assert!(cache_fresh(5_000, 1_000));

        let mut cache = CollectionsCache::default();
        cache.repos.insert(
            "a/b".into(),
            RepoCache {
                fetched_at: 42,
                branch: "main".into(),
                tree: crate::collections::github::parse_tree(
                    r#"{"sha": "x", "tree": []}"#,
                )
                .unwrap(),
            },
        );
        save_cache(&dir, &cache).unwrap();
        let loaded = load_cache(&dir);
        assert_eq!(loaded.repos["a/b"].branch, "main");
        assert_eq!(loaded.repos["a/b"].fetched_at, 42);

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn malformed_store_files_read_as_empty() {
        let dir = tmp_dir("malformed");
        std::fs::write(dir.join(STORE_FILE), "{ not json").unwrap();
        std::fs::write(dir.join(CACHE_FILE), "{ not json").unwrap();
        assert!(load_user_collections(&dir).is_empty());
        assert!(load_cache(&dir).repos.is_empty());

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
```

Run: `cd src-tauri && cargo test collections::store`
Expected: FAIL.

- [ ] **Step 3: Implement `store.rs`**

```rust
use super::github::TreeResponse;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub const STORE_FILE: &str = "collections.json";
pub const CACHE_FILE: &str = "collections-cache.json";
pub const CACHE_TTL_SECS: u64 = 24 * 60 * 60;

/// A collection the user added by URL, persisted in app_config_dir.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserCollection {
    pub id: String,
    pub title: String,
    pub owner: String,
    pub repo: String,
    #[serde(default)]
    pub subpath: Option<String>,
    pub added_at: u64,
}

/// One repo's enumeration, cached so browse + install share a single
/// tree fetch and the API rate limit is not a practical constraint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoCache {
    pub fetched_at: u64,
    pub branch: String,
    pub tree: TreeResponse,
}

/// The last successfully fetched catalog manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestCache {
    pub fetched_at: u64,
    pub raw: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CollectionsCache {
    pub manifest: Option<ManifestCache>,
    pub repos: HashMap<String, RepoCache>,
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn cache_fresh(fetched_at: u64, now: u64) -> bool {
    now.saturating_sub(fetched_at) < CACHE_TTL_SECS
}

/// A missing or malformed store reads as empty — browsing must survive
/// a hand-edited config file.
pub fn load_user_collections(dir: &Path) -> Vec<UserCollection> {
    let Ok(raw) = fs::read_to_string(dir.join(STORE_FILE)) else {
        return Vec::new();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

pub fn save_user_collections(dir: &Path, collections: &[UserCollection]) -> Result<(), String> {
    let raw = serde_json::to_string_pretty(collections).map_err(|e| e.to_string())?;
    fs::write(dir.join(STORE_FILE), raw).map_err(|e| e.to_string())
}

pub fn load_cache(dir: &Path) -> CollectionsCache {
    let Ok(raw) = fs::read_to_string(dir.join(CACHE_FILE)) else {
        return CollectionsCache::default();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

pub fn save_cache(dir: &Path, cache: &CollectionsCache) -> Result<(), String> {
    let raw = serde_json::to_string_pretty(cache).map_err(|e| e.to_string())?;
    fs::write(dir.join(CACHE_FILE), raw).map_err(|e| e.to_string())
}

/// Validate the slug, refuse duplicates, persist, and return the entry.
pub fn add_user_collection(
    dir: &Path,
    repo: &str,
    title: Option<String>,
) -> Result<UserCollection, String> {
    let trimmed = repo.trim().trim_end_matches('/');
    let Some((owner, name)) = super::split_repo(trimmed) else {
        return Err(format!("'{repo}' is not a valid owner/repo repository slug"));
    };
    let mut stored = load_user_collections(dir);
    if stored.iter().any(|c| c.owner == owner && c.repo == name) {
        return Err(format!("'{owner}/{name}' is already in your collections"));
    }
    let entry = UserCollection {
        id: format!("{owner}/{name}"),
        title: title.unwrap_or_else(|| name.clone()),
        owner,
        repo: name,
        subpath: None,
        added_at: now_secs(),
    };
    stored.push(entry.clone());
    save_user_collections(dir, &stored)?;
    Ok(entry)
}

pub fn remove_user_collection(dir: &Path, id: &str) -> Result<(), String> {
    let mut stored = load_user_collections(dir);
    let before = stored.len();
    stored.retain(|c| c.id != id);
    if stored.len() == before {
        return Err(format!("no user collection named '{id}'"));
    }
    save_user_collections(dir, &stored)
}
```

- [ ] **Step 4: Failing tests for the catalog chain and merge**

Append to `src-tauri/src/collections/manifest.rs` tests:

```rust
    use crate::collections::github::testutil::FakeGithubHttp;
    use crate::collections::store::{CollectionsCache, UserCollection};
    use crate::collections::CollectionInfo;

    const MANIFEST_BODY: &str = r#"{"version": 1, "collections": [
        {"id": "anthropics-skills", "title": "Anthropic Skills", "repo": "anthropics/skills"}
    ]}"#;

    #[test]
    fn load_catalog_prefers_the_remote_manifest_and_caches_it() {
        let mut http = FakeGithubHttp::new();
        http.mount("collections.json", MANIFEST_BODY);
        let mut cache = CollectionsCache::default();

        let (cols, source) = load_catalog(&http, &mut cache, 1_000);
        assert_eq!(cols.len(), 1);
        assert_eq!(source, CatalogSource::Manifest);
        assert!(cache.manifest.as_ref().unwrap().raw.contains("anthropics"));

        // offline afterwards: the cached manifest serves
        let (cols, source) = load_catalog(&FakeGithubHttp::new(), &mut cache, 2_000);
        assert_eq!(cols.len(), 1);
        assert_eq!(source, CatalogSource::Cached);
    }

    #[test]
    fn load_catalog_falls_back_to_the_bundled_copy() {
        let (cols, source) = load_catalog(&FakeGithubHttp::new(), &mut CollectionsCache::default(), 1_000);
        assert_eq!(source, CatalogSource::Bundled);
        assert!(cols.len() >= 3);
    }

    #[test]
    fn merge_dedupes_user_added_repos_already_in_the_manifest() {
        let manifest = parse_manifest(MANIFEST_BODY).unwrap();
        let user = vec![UserCollection {
            id: "anthropics/skills".into(),
            title: "dupe".into(),
            owner: "anthropics".into(),
            repo: "skills".into(),
            subpath: None,
            added_at: 0,
        }, UserCollection {
            id: "my/own".into(),
            title: "own".into(),
            owner: "my".into(),
            repo: "own".into(),
            subpath: None,
            added_at: 0,
        }];
        let merged: Vec<CollectionInfo> = merge_collections(&manifest, &user);
        assert_eq!(merged.len(), 2);
        assert!(merged.iter().all(|c| c.builtin == (c.repo != "own")));
    }
```

Run: `cd src-tauri && cargo test collections::manifest`
Expected: new tests FAIL (`load_catalog`/`merge_collections` undefined).

- [ ] **Step 5: Implement the chain and merge**

Add to `src-tauri/src/collections/manifest.rs`:

```rust
use super::github::GithubHttp;
use super::store::{CollectionsCache, UserCollection};
use super::CollectionInfo;

/// Remote manifest → cached manifest → bundled fallback. A tampered or
/// broken manifest can at worst advertise unhelpful repos; it names
/// repositories only and cannot bypass any install validation.
pub fn load_catalog(
    http: &dyn GithubHttp,
    cache: &mut CollectionsCache,
    now: u64,
) -> (Vec<ManifestCollection>, CatalogSource) {
    if let Ok(text) = http.get_text(CATALOG_URL) {
        if let Ok(collections) = parse_manifest(&text) {
            cache.manifest = Some(super::store::ManifestCache {
                fetched_at: now,
                raw: text,
            });
            return (collections, CatalogSource::Manifest);
        }
    }
    if let Some(cached) = &cache.manifest {
        if let Ok(collections) = parse_manifest(&cached.raw) {
            return (collections, CatalogSource::Cached);
        }
    }
    (
        parse_manifest(BUNDLED_MANIFEST).unwrap_or_default(),
        CatalogSource::Bundled,
    )
}

/// Manifest entries first; user-added collections appended unless the
/// same repo is already offered. Dedupe key is owner + repo.
pub fn merge_collections(
    manifest: &[ManifestCollection],
    user: &[UserCollection],
) -> Vec<CollectionInfo> {
    let mut out: Vec<CollectionInfo> = manifest
        .iter()
        .filter_map(|c| {
            let (owner, repo) = super::split_repo(&c.repo)?;
            Some(CollectionInfo {
                id: c.id.clone(),
                title: c.title.clone(),
                owner,
                repo,
                subpath: c.subpath.clone(),
                builtin: true,
                skill_count: None,
            })
        })
        .collect();

    for u in user {
        if out.iter().any(|c| c.owner == u.owner && c.repo == u.repo) {
            continue;
        }
        out.push(CollectionInfo {
            id: u.id.clone(),
            title: u.title.clone(),
            owner: u.owner.clone(),
            repo: u.repo.clone(),
            subpath: u.subpath.clone(),
            builtin: false,
            skill_count: None,
        });
    }
    out
}
```

Update `mod.rs` exports:

```rust
pub use manifest::{
    load_catalog, merge_collections, parse_manifest, CatalogSource, ManifestCollection,
    BUNDLED_MANIFEST, CATALOG_URL,
};
```

- [ ] **Step 6: Run the tests**

Run: `cd src-tauri && cargo test collections`
Expected: PASS (all of Tasks 1–4).

- [ ] **Step 7: Gate and commit**

Run: `cd src-tauri && cargo fmt && cargo clippy --all-targets && cargo test`

```bash
git add src-tauri/src/collections
git commit -m "feat(collections): persist user collections and the browse cache"
```

---

### Task 5: Install core (`install.rs`) — the security-critical path

**Files:**
- Create: `src-tauri/src/collections/install.rs`
- Modify: `src-tauri/src/commands/create_skill.rs:23` (`fn validate_skill_folder_name` → `pub(crate) fn`)
- Modify: `src-tauri/src/commands/mod.rs:71` (`fn validate_manifest_at` → `pub(crate) fn`) and add `pub(crate) use create_skill::validate_skill_folder_name;`
- Modify: `src-tauri/src/collections/mod.rs`

**Interfaces:**
- Consumes: `GithubHttp`, `TreeResponse`, `validate_skill_folder_name`, `validate_manifest_at`, `skills_roots` (command layer supplies roots).
- Produces: `RemoteFile { relative_path, sha }`, `Provenance { owner, repo, path, branch, tree_sha, installed_at, collection_id }`, `pub fn safe_relative(rel: &str) -> bool`, `pub fn files_for_skill(tree: &TreeResponse, folder: &str) -> Result<(Vec<RemoteFile>, usize), String>` (files + skipped link/submodule count), `pub fn install_skill_files(http: &dyn GithubHttp, roots: &[PathBuf], root: &Path, name: &str, files: &[RemoteFile], provenance: &Provenance, overwrite: bool) -> Result<PathBuf, String>` returning the manifest path.

- [ ] **Step 1: Widen visibility of the shared validators**

In `src-tauri/src/commands/create_skill.rs` change:

```rust
fn validate_skill_folder_name(name: &str) -> Result<(), String> {
```
to
```rust
pub(crate) fn validate_skill_folder_name(name: &str) -> Result<(), String> {
```

In `src-tauri/src/commands/mod.rs` change:

```rust
fn validate_manifest_at(path: &Path, roots: &[PathBuf]) -> bool {
```
to
```rust
pub(crate) fn validate_manifest_at(path: &Path, roots: &[PathBuf]) -> bool {
```

and add next to the `pub use create_skill::...` block (line 16):

```rust
pub(crate) use create_skill::validate_skill_folder_name;
```

- [ ] **Step 2: Write the failing tests**

Create `src-tauri/src/collections/install.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::github::testutil::FakeGithubHttp;
    use crate::collections::github::parse_tree;
    use std::fs;
    use std::path::{Path, PathBuf};

    const TREE_JSON: &str = r#"{"sha": "tree-sha", "tree": [
        {"path": "skills/pdf/SKILL.md", "mode": "100644", "sha": "sha-skill", "type": "blob"},
        {"path": "skills/pdf/scripts/run.py", "mode": "100755", "sha": "sha-script", "type": "blob"},
        {"path": "skills/pdf/link", "mode": "120000", "sha": "sha-link", "type": "blob"},
        {"path": "other/README.md", "mode": "100644", "sha": "sha-other", "type": "blob"}
    ]}"#;

    fn provenance() -> Provenance {
        Provenance {
            owner: "anthropics".into(),
            repo: "skills".into(),
            path: "skills/pdf".into(),
            branch: "main".into(),
            tree_sha: "tree-sha".into(),
            installed_at: 1_000,
            collection_id: "anthropics-skills".into(),
        }
    }

    fn http_with_blobs() -> FakeGithubHttp {
        let mut http = FakeGithubHttp::new();
        http.mount("blobs/sha-skill", r#"{"content": "LS1tLQpuYW1lOiBwZGY=", "encoding": "base64"}"#);
        http.mount("blobs/sha-script", r#"{"content": "cHJpbnQoJ2hpJyk=", "encoding": "base64"}"#);
        http
    }

    fn dirs(tag: &str) -> (PathBuf, PathBuf) {
        let base = std::env::temp_dir()
            .join(format!("skill-manager-install-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("managed")).unwrap();
        (base.clone(), base.join("managed"))
    }

    #[test]
    fn safe_relative_rejects_traversal() {
        assert!(safe_relative("SKILL.md"));
        assert!(safe_relative("scripts/run.py"));
        for bad in [
            "", "/abs", "../up", "a/../../b", "a//b", "a/./b", "..", "a\\b", "a/../b",
        ] {
            assert!(!safe_relative(bad), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn files_for_skill_selects_folder_blobs_and_skips_links() {
        let tree = parse_tree(TREE_JSON).unwrap();
        let (files, skipped) = files_for_skill(&tree, "skills/pdf").unwrap();
        assert_eq!(skipped, 1); // the 120000 symlink entry
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|f| f.relative_path == "SKILL.md"));
        assert!(files.iter().any(|f| f.relative_path == "scripts/run.py"));
    }

    #[test]
    fn files_for_skill_requires_a_manifest() {
        let tree = parse_tree(TREE_JSON).unwrap();
        let err = files_for_skill(&tree, "other").unwrap_err();
        assert!(err.contains("no SKILL.md"), "unexpected error: {err}");
    }

    #[test]
    fn files_for_skill_root_folder_takes_the_whole_repo() {
        let root_tree = r#"{"sha": "x", "tree": [
            {"path": "SKILL.md", "mode": "100644", "sha": "s", "type": "blob"}
        ]}"#;
        let (files, _) = files_for_skill(&parse_tree(root_tree).unwrap(), "").unwrap();
        assert_eq!(files[0].relative_path, "SKILL.md");
    }

    #[test]
    fn install_writes_files_provenance_and_rejects_unmanaged_roots() {
        let (_base, managed) = dirs("happy");
        let unmanaged = managed.parent().unwrap().join("unmanaged");
        let roots = vec![managed.clone()];
        let tree = parse_tree(TREE_JSON).unwrap();
        let (files, _) = files_for_skill(&tree, "skills/pdf").unwrap();
        let http = http_with_blobs();

        let err = install_skill_files(&http, &roots, &unmanaged, "pdf", &files, &provenance(), false).unwrap_err();
        assert!(err.contains("not managed"), "unexpected error: {err}");

        let manifest = install_skill_files(&http, &roots, &managed, "pdf", &files, &provenance(), false).unwrap();
        assert!(manifest.is_file());
        assert!(managed.join("pdf/scripts/run.py").is_file());
        let prov: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(managed.join("pdf/").join(PROVENANCE_FILE)).unwrap(),
        )
        .unwrap();
        assert_eq!(prov["treeSha"], "tree-sha");
        assert_eq!(prov["collectionId"], "anthropics-skills");
        // no temp dirs left behind
        let leftovers: Vec<_> = fs::read_dir(&managed).unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp-"))
            .collect();
        assert!(leftovers.is_empty());

        let _ = fs::remove_dir_all(managed.parent().unwrap());
    }

    #[test]
    fn install_reports_collisions_and_overwrite_removes_only_skills() {
        let (_base, managed) = dirs("collision");
        let roots = vec![managed.clone()];
        let tree = parse_tree(TREE_JSON).unwrap();
        let (files, _) = files_for_skill(&tree, "skills/pdf").unwrap();
        let http = http_with_blobs();
        install_skill_files(&http, &roots, &managed, "pdf", &files, &provenance(), false).unwrap();

        let err = install_skill_files(&http, &roots, &managed, "pdf", &files, &provenance(), false).unwrap_err();
        assert!(err.contains("already exists"), "unexpected error: {err}");

        // overwrite replaces the skill (same content → same tree sha)
        install_skill_files(&http, &roots, &managed, "pdf", &files, &provenance(), true).unwrap();

        // overwrite refuses to delete something that is not a managed skill shape
        fs::create_dir_all(managed.join("not-a-skill")).unwrap();
        fs::write(managed.join("not-a-skill/notes.txt"), "x").unwrap();
        let err = install_skill_files(&http, &roots, &managed, "not-a-skill", &files, &provenance(), true).unwrap_err();
        assert!(err.contains("not a skill this app manages"), "unexpected error: {err}");
        assert!(managed.join("not-a-skill/notes.txt").is_file());

        let _ = fs::remove_dir_all(managed.parent().unwrap());
    }

    #[test]
    fn install_validates_the_folder_name() {
        let (_base, managed) = dirs("name");
        let roots = vec![managed.clone()];
        let tree = parse_tree(TREE_JSON).unwrap();
        let (files, _) = files_for_skill(&tree, "skills/pdf").unwrap();
        let http = http_with_blobs();
        for bad in ["../escape", ".disabled", "a b"] {
            assert!(install_skill_files(&http, &roots, &managed, bad, &files, &provenance(), false).is_err());
        }

        let _ = fs::remove_dir_all(managed.parent().unwrap());
    }

    #[test]
    fn failed_downloads_leave_no_partial_folder() {
        let (_base, managed) = dirs("partial");
        let roots = vec![managed.clone()];
        let tree = parse_tree(TREE_JSON).unwrap();
        let (files, _) = files_for_skill(&tree, "skills/pdf").unwrap();
        // no fixtures mounted → every blob fetch fails
        let http = FakeGithubHttp::new();
        let err = install_skill_files(&http, &roots, &managed, "pdf", &files, &provenance(), false).unwrap_err();
        assert!(err.contains("no fixture") || err.contains("GitHub"), "unexpected error: {err}");
        assert!(!managed.join("pdf").exists());

        let _ = fs::remove_dir_all(managed.parent().unwrap());
    }
}
```

Run: `cd src-tauri && cargo test collections::install`
Expected: FAIL.

- [ ] **Step 3: Implement**

```rust
use super::github::GithubHttp;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Where install provenance lives inside every installed skill folder.
pub const PROVENANCE_FILE: &str = ".collection-source.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteFile {
    /// Path relative to the skill folder — checked for safety before any write.
    pub relative_path: String,
    pub sha: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Provenance {
    pub owner: String,
    pub repo: String,
    pub path: String,
    pub branch: String,
    pub tree_sha: String,
    pub installed_at: u64,
    pub collection_id: String,
}

/// A remote path is writable only if it is a plain relative path with
/// no dot components — GitHub tree paths use `/` separators only.
pub fn safe_relative(rel: &str) -> bool {
    !rel.is_empty()
        && !rel.starts_with('/')
        && !rel.contains('\\')
        && rel.split('/').all(|part| !part.is_empty() && part != "." && part != "..")
}

/// The blobs of one skill folder, relative to it. Symlinks (mode
/// 120000) and submodules (160000) are skipped and counted — they are
/// never materialized on disk. `folder == ""` means the whole repo is
/// the skill.
pub fn files_for_skill(tree: &super::github::TreeResponse, folder: &str) -> Result<(Vec<RemoteFile>, usize), String> {
    let prefix = if folder.is_empty() {
        String::new()
    } else {
        format!("{folder}/")
    };
    let mut files = Vec::new();
    let mut skipped = 0;
    let mut has_manifest = false;
    for entry in &tree.tree {
        if entry.kind != "blob" {
            continue;
        }
        let Some(rel) = entry.path.strip_prefix(prefix.as_str()) else {
            continue;
        };
        if entry.mode == "120000" || entry.mode == "160000" {
            skipped += 1;
            continue;
        }
        if !safe_relative(rel) {
            return Err(format!("remote path '{rel}' is not safe to write"));
        }
        if rel == "SKILL.md" {
            has_manifest = true;
        }
        files.push(RemoteFile {
            relative_path: rel.to_string(),
            sha: entry.sha.clone(),
        });
    }
    if !has_manifest {
        return Err(format!("'{folder}' has no SKILL.md"));
    }
    Ok((files, skipped))
}

/// Download `files` into `<root>/<name>/` and return the manifest
/// path. Mirrors create_skill's security model: `root` must be exactly
/// one of the managed `roots`, `name` passes the folder-name policy,
/// collisions are explicit, and overwriting only removes folders whose
/// manifest passes `validate_manifest_at`. Everything is built in a
/// temp dir and renamed in, so a failed download leaves nothing behind.
pub fn install_skill_files(
    http: &dyn GithubHttp,
    roots: &[PathBuf],
    root: &Path,
    name: &str,
    files: &[RemoteFile],
    provenance: &Provenance,
    overwrite: bool,
) -> Result<PathBuf, String> {
    if !roots.iter().any(|r| r == root) {
        return Err("target folder is not managed by this app".into());
    }
    crate::commands::validate_skill_folder_name(name)?;

    let target = root.join(name);
    if target.exists() {
        if !overwrite {
            return Err(format!("a skill named '{name}' already exists in this folder"));
        }
        if !crate::commands::validate_manifest_at(&target.join("SKILL.md"), roots) {
            return Err(format!(
                "'{name}' exists but is not a skill this app manages; remove it manually first"
            ));
        }
        fs::remove_dir_all(&target).map_err(|e| format!("cannot remove existing skill: {e}"))?;
    }

    fs::create_dir_all(root).map_err(|e| format!("cannot create target folder: {e}"))?;
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let tmp = root.join(format!(".{name}.tmp-{}-{nanos}", std::process::id()));

    let build = || -> Result<(), String> {
        for file in files {
            let bytes = http.fetch_blob(&provenance.owner, &provenance.repo, &file.sha)?;
            let dest = tmp.join(&file.relative_path);
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent).map_err(|e| format!("cannot create folders: {e}"))?;
            }
            fs::write(&dest, bytes).map_err(|e| format!("cannot write file: {e}"))?;
        }
        let prov = serde_json::to_string_pretty(provenance)
            .map_err(|e| format!("cannot serialize provenance: {e}"))?;
        fs::write(tmp.join(PROVENANCE_FILE), format!("{prov}\n"))
            .map_err(|e| format!("cannot write provenance: {e}"))?;
        Ok(())
    };
    if let Err(e) = build() {
        let _ = fs::remove_dir_all(&tmp);
        return Err(e);
    }
    if let Err(e) = fs::rename(&tmp, &target) {
        let _ = fs::remove_dir_all(&tmp);
        return Err(format!("cannot finalize install: {e}"));
    }
    Ok(target.join("SKILL.md"))
}
```

Add to `src-tauri/src/collections/mod.rs`:

```rust
pub mod install;

pub use install::{files_for_skill, install_skill_files, safe_relative, Provenance, RemoteFile, PROVENANCE_FILE};
```

- [ ] **Step 4: Run the tests**

Run: `cd src-tauri && cargo test collections && cargo test commands`
Expected: PASS (new tests plus all pre-existing command tests — the visibility change is behavior-neutral).

- [ ] **Step 5: Gate and commit**

Run: `cd src-tauri && cargo fmt && cargo clippy --all-targets && cargo test`

```bash
git add src-tauri/src/collections src-tauri/src/commands
git commit -m "feat(collections): install remote skill folders through the managed-root checks"
```

---

### Task 6: Tauri commands + registration

**Files:**
- Create: `src-tauri/src/commands/collections.rs`
- Modify: `src-tauri/src/commands/mod.rs` (module + re-exports + `resolve_target_root`)
- Modify: `src-tauri/src/commands/create_skill.rs` (use `resolve_target_root`)
- Modify: `src-tauri/src/skills/mod.rs:151` (`fn parse_frontmatter` → `pub(crate) fn`)
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: everything from Tasks 1–5; `skills_roots`, `find_skill_by_manifest`, projects store.
- Produces (frontend contract, camelCase on the wire): commands `list_collections() -> ListCollectionsResult { collections: CollectionInfo[], source }`, `browse_collection(id: string) -> RemoteSkill[]`, `refresh_collection(id: string) -> RemoteSkill[]`, `add_collection(repo: string, title: string | null) -> CollectionInfo`, `remove_collection(id: string) -> void`, `install_skill(tool, scope, projectPath: string | null, skill: RemoteSkill, collectionId: string, overwrite: bool | null) -> InstallResult { skill: Skill, skippedLinks: number }`, `fetch_skill_manifest(skill: RemoteSkill) -> SkillManifest { name, description: string | null }`. Also `pub(crate) fn resolve_target_root(...)` in `commands/mod.rs`.

- [ ] **Step 1: Extract shared root resolution**

In `src-tauri/src/commands/mod.rs` add:

```rust
/// Resolve the destination skills root for a tool + scope selection —
/// the shared front half of create_skill and install_skill. Project
/// scope requires an already-tracked project, exactly like create_skill.
pub(crate) fn resolve_target_root(
    tracked: &[ProjectInfo],
    tool: crate::skills::AgentTool,
    scope: crate::skills::SkillScope,
    project_path: Option<String>,
) -> Result<PathBuf, String> {
    let adapter = crate::skills::adapter_for(tool);
    match scope {
        crate::skills::SkillScope::User => Ok(adapter.skills_dir()),
        crate::skills::SkillScope::Project => {
            let Some(path) = project_path else {
                return Err("project scope requires a project".into());
            };
            if !tracked.iter().any(|p| p.path == path) {
                return Err("project is not tracked by this app".into());
            }
            Ok(PathBuf::from(path).join(adapter.project_subpath()))
        }
    }
}
```

Then in `src-tauri/src/commands/create_skill.rs`, replace lines 126–139 (the root-resolution `match` inside `create_skill`) with:

```rust
    let tracked = projects::list(&app).unwrap_or_default();
    let root = resolve_target_root(&tracked, tool, scope, project_path)?;
```

(Keep the surrounding `let manifest = ...` and later lines unchanged. Run `cargo test` — create_skill's existing behavior is identical.)

- [ ] **Step 2: Widen `parse_frontmatter`**

In `src-tauri/src/skills/mod.rs` change `fn parse_frontmatter(raw: &str) -> (String, String)` to `pub(crate) fn parse_frontmatter(raw: &str) -> (String, String)`.

- [ ] **Step 3: Write the command module**

Create `src-tauri/src/commands/collections.rs`:

```rust
use super::{find_skill_by_manifest, resolve_target_root, skills_roots};
use crate::collections::github::UreqGithubHttp;
use crate::collections::{
    self, CatalogSource, CollectionInfo, CollectionsCache, ManifestCollection, Provenance,
    RemoteSkill,
};
use crate::projects;
use crate::skills::{self, Skill};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

const HTTP: UreqGithubHttp = UreqGithubHttp;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListCollectionsResult {
    pub collections: Vec<CollectionInfo>,
    pub source: CatalogSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallResult {
    pub skill: Skill,
    pub skipped_links: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillManifest {
    pub name: String,
    pub description: Option<String>,
}

fn config_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn catalog_and_user(
    app: &AppHandle,
    cache: &mut CollectionsCache,
) -> Result<(Vec<ManifestCollection>, Vec<collections::UserCollection>), String> {
    let dir = config_dir(app)?;
    let (manifest, _) = collections::load_catalog(&HTTP, cache, collections::now_secs());
    Ok((manifest, collections::load_user_collections(&dir)))
}

fn find_collection(
    app: &AppHandle,
    cache: &mut CollectionsCache,
    id: &str,
) -> Result<CollectionInfo, String> {
    let (manifest, user) = catalog_and_user(app, cache)?;
    collections::merge_collections(&manifest, &user)
        .into_iter()
        .find(|c| c.id == id)
        .ok_or_else(|| format!("no collection named '{id}'"))
}

/// One repo's `(branch, tree, from_cache)` with cache-first semantics;
/// `force` bypasses freshness for the refresh button. When the network
/// fails but a stale cache entry exists, the stale tree is served —
/// browsing degrades instead of erroring (the spec's fallback chain).
fn repo_tree(
    app: &AppHandle,
    cache: &mut CollectionsCache,
    owner: &str,
    repo: &str,
    force: bool,
) -> Result<(String, collections::TreeResponse, bool), String> {
    let key = format!("{owner}/{repo}");
    if !force {
        if let Some(hit) = cache.repos.get(&key) {
            if collections::cache_fresh(hit.fetched_at, collections::now_secs()) {
                return Ok((hit.branch.clone(), hit.tree.clone(), true));
            }
        }
    }
    match collections::enumerate_repo_skills(&HTTP, owner, repo, None) {
        Ok((branch, tree, _)) => {
            cache.repos.insert(
                key,
                collections::RepoCache {
                    fetched_at: collections::now_secs(),
                    branch: branch.clone(),
                    tree: tree.clone(),
                },
            );
            let _ = collections::save_cache(&config_dir(app)?, cache);
            Ok((branch, tree, false))
        }
        Err(e) => match cache.repos.get(&key) {
            Some(hit) => Ok((hit.branch.clone(), hit.tree.clone(), true)),
            None => Err(e),
        },
    }
}

#[tauri::command]
pub fn list_collections(app: AppHandle) -> Result<ListCollectionsResult, String> {
    let dir = config_dir(&app)?;
    let mut cache = collections::load_cache(&dir);
    let (manifest, source) = collections::load_catalog(&HTTP, &mut cache, collections::now_secs());
    let _ = collections::save_cache(&dir, &cache);
    let user = collections::load_user_collections(&dir);
    let mut merged = collections::merge_collections(&manifest, &user);
    for collection in &mut merged {
        let key = format!("{}/{}", collection.owner, collection.repo);
        if let Some(hit) = cache.repos.get(&key) {
            let skills = collections::skills_from_tree(
                &collection.owner,
                &collection.repo,
                &hit.branch,
                &hit.tree,
                collection.subpath.as_deref(),
            );
            collection.skill_count = Some(skills.len() as u64);
        }
    }
    Ok(ListCollectionsResult {
        collections: merged,
        source,
    })
}

fn browse(app: AppHandle, id: String, force: bool) -> Result<Vec<RemoteSkill>, String> {
    let dir = config_dir(&app)?;
    let mut cache = collections::load_cache(&dir);
    let info = find_collection(&app, &mut cache, &id)?;
    let (branch, tree, _) = repo_tree(&app, &mut cache, &info.owner, &info.repo, force)?;
    Ok(collections::skills_from_tree(
        &info.owner,
        &info.repo,
        &branch,
        &tree,
        info.subpath.as_deref(),
    ))
}

#[tauri::command]
pub fn browse_collection(app: AppHandle, id: String) -> Result<Vec<RemoteSkill>, String> {
    browse(app, id, false)
}

#[tauri::command]
pub fn refresh_collection(app: AppHandle, id: String) -> Result<Vec<RemoteSkill>, String> {
    browse(app, id, true)
}

#[tauri::command]
pub fn add_collection(
    app: AppHandle,
    repo: String,
    title: Option<String>,
) -> Result<CollectionInfo, String> {
    let dir = config_dir(&app)?;
    let entry = collections::add_user_collection(&dir, &repo, title)?;
    // Probe: reachability now, so add-time feedback is honest.
    HTTP.fetch_default_branch(&entry.owner, &entry.repo)?;
    Ok(CollectionInfo {
        id: entry.id,
        title: entry.title,
        owner: entry.owner,
        repo: entry.repo,
        subpath: entry.subpath,
        builtin: false,
        skill_count: None,
    })
}

#[tauri::command]
pub fn remove_collection(app: AppHandle, id: String) -> Result<(), String> {
    let dir = config_dir(&app)?;
    let mut cache = collections::load_cache(&dir);
    let info = find_collection(&app, &mut cache, &id)?;
    if info.builtin {
        return Err("built-in collections cannot be removed".into());
    }
    collections::remove_user_collection(&dir, &id)
}

#[tauri::command]
pub fn install_skill(
    app: AppHandle,
    tool: skills::AgentTool,
    scope: skills::SkillScope,
    project_path: Option<String>,
    skill: RemoteSkill,
    collection_id: String,
    overwrite: Option<bool>,
) -> Result<InstallResult, String> {
    let tracked = projects::list(&app).unwrap_or_default();
    let root = resolve_target_root(&tracked, tool, scope, project_path)?;
    let roots = skills_roots(&tracked);

    let dir = config_dir(&app)?;
    let mut cache = collections::load_cache(&dir);
    let info = find_collection(&app, &mut cache, &collection_id)?;
    let (branch, tree, _) = repo_tree(&app, &mut cache, &skill.owner, &skill.repo, false)?;

    let (files, skipped_links) = collections::files_for_skill(&tree, &skill.path)?;
    let provenance = Provenance {
        owner: skill.owner.clone(),
        repo: skill.repo.clone(),
        path: skill.path.clone(),
        branch,
        tree_sha: tree.sha.clone(),
        installed_at: collections::now_secs(),
        collection_id,
    };
    let manifest = collections::install_skill_files(
        &HTTP,
        &roots,
        &root,
        &skill.name,
        &files,
        &provenance,
        overwrite.unwrap_or(false),
    )?;

    if let Some(project) = tracked.iter().find(|p| manifest.starts_with(&p.path)) {
        let _ = projects::clear_skill_count(&app, &project.path);
    }
    let installed = find_skill_by_manifest(&app, &manifest)
        .ok_or_else(|| "skill not found after installation".to_string())?;
    Ok(InstallResult {
        skill: installed,
        skipped_links: skipped_links as u64,
    })
}

#[tauri::command]
pub fn fetch_skill_manifest(app: AppHandle, skill: RemoteSkill) -> Result<SkillManifest, String> {
    let dir = config_dir(&app)?;
    let mut cache = collections::load_cache(&dir);
    let (_, tree, _) = repo_tree(&app, &mut cache, &skill.owner, &skill.repo, false)?;
    let manifest_path = if skill.path.is_empty() {
        "SKILL.md".to_string()
    } else {
        format!("{}/SKILL.md", skill.path)
    };
    let Some(entry) = tree.tree.iter().find(|e| e.path == manifest_path) else {
        return Err(format!("no SKILL.md found at '{manifest_path}'"));
    };
    let bytes = HTTP.fetch_blob(&skill.owner, &skill.repo, &entry.sha)?;
    let text = String::from_utf8(bytes).map_err(|_| "SKILL.md is not valid UTF-8".to_string())?;
    let (name, description) = skills::parse_frontmatter(&text);
    Ok(SkillManifest {
        name: if name.is_empty() { skill.name.clone() } else { name },
        description: if description.is_empty() { None } else { Some(description) },
    })
}
```

- [ ] **Step 4: Wire the module and handler**

In `src-tauri/src/commands/mod.rs`:

```rust
mod collections;
```

(added beside `mod create_skill;`) and extend the re-export block:

```rust
pub use collections::{
    __cmd__add_collection, __cmd__browse_collection, __cmd__fetch_skill_manifest,
    __cmd__install_skill, __cmd__list_collections, __cmd__refresh_collection,
    __cmd__remove_collection, __tauri_command_name_add_collection,
    __tauri_command_name_browse_collection, __tauri_command_name_fetch_skill_manifest,
    __tauri_command_name_install_skill, __tauri_command_name_list_collections,
    __tauri_command_name_refresh_collection, __tauri_command_name_remove_collection,
    add_collection, browse_collection, fetch_skill_manifest, install_skill,
    list_collections, refresh_collection, remove_collection,
};
```

In `src-tauri/src/lib.rs`, add to `generate_handler![...]` after `commands::create_skill,`:

```rust
            commands::list_collections,
            commands::browse_collection,
            commands::refresh_collection,
            commands::add_collection,
            commands::remove_collection,
            commands::install_skill,
            commands::fetch_skill_manifest,
```

- [ ] **Step 5: Gate and commit**

Run: `cd src-tauri && cargo fmt && cargo clippy --all-targets && cargo test`
Expected: clean and green (no new unit tests here — the logic was tested in Tasks 1–5; commands are thin composition).

```bash
git add src-tauri/src
git commit -m "feat(collections): expose browse/install tauri commands"
```

---

### Task 7: Frontend types, API client, search helper

**Files:**
- Create: `src/types/collection.ts`
- Modify: `src/types/index.ts`
- Create: `src/api/collections.ts`
- Modify: `src/api/index.ts`
- Create: `src/utils/collectionSearch.ts`
- Create: `src/utils/__tests__/collectionSearch.test.ts`

**Interfaces:**
- Consumes: command names/args from Task 6; `Skill`, `AgentTool`, `SkillScope` from existing types.
- Produces (TS): `RemoteSkill`, `CollectionInfo`, `CatalogSource`, `ListCollectionsResult`, `SkillManifest`, `InstallResult`, `collectionsApi` (methods `listCollections`, `browseCollection`, `refreshCollection`, `addCollection`, `removeCollection`, `installSkill`, `fetchSkillManifest`), `searchRemoteSkills(skills, query)`.

- [ ] **Step 1: Write the failing test**

Create `src/utils/__tests__/collectionSearch.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { searchRemoteSkills } from "../collectionSearch";
import type { RemoteSkill } from "../../types";

function remote(name: string, description: string | null): RemoteSkill {
  return { name, description, owner: "anthropics", repo: "skills", path: `skills/${name}`, branch: "main" };
}

describe("searchRemoteSkills", () => {
  const skills = [
    remote("zebra", null),
    remote("apple", "a fruit skill"),
    remote("Banana", "also yellow"),
  ];

  it("returns everything sorted by name when the query is empty", () => {
    expect(searchRemoteSkills(skills, "").map((s) => s.name)).toEqual(["apple", "Banana", "zebra"]);
  });

  it("matches names case-insensitively", () => {
    expect(searchRemoteSkills(skills, "BAN").map((s) => s.name)).toEqual(["Banana"]);
  });

  it("matches descriptions when loaded", () => {
    expect(searchRemoteSkills(skills, "yellow").map((s) => s.name)).toEqual(["Banana"]);
  });

  it("treats unloaded descriptions as empty", () => {
    expect(searchRemoteSkills(skills, "fruit")).toEqual([]);
  });
});
```

Run: `npm test`
Expected: FAIL — module not found.

- [ ] **Step 2: Types**

Create `src/types/collection.ts`:

```ts
import type { Skill, SkillScope } from "./skill";
import type { AgentTool } from "./tool";

/** One installable skill in a remote GitHub collection. */
export interface RemoteSkill {
  name: string;
  /** Filled lazily from the remote SKILL.md via fetchSkillManifest. */
  description: string | null;
  owner: string;
  repo: string;
  /** Folder containing the SKILL.md; "" when the repo root is the skill. */
  path: string;
  branch: string;
}

/** One browsable collection, built-in (manifest) or user-added. */
export interface CollectionInfo {
  id: string;
  title: string;
  owner: string;
  repo: string;
  subpath: string | null;
  builtin: boolean;
  skillCount: number | null;
}

export type CatalogSource = "manifest" | "cached" | "bundled";

export interface ListCollectionsResult {
  collections: CollectionInfo[];
  source: CatalogSource;
}

/** Frontmatter read from a remote SKILL.md. */
export interface SkillManifest {
  name: string;
  description: string | null;
}

export interface InstallResult {
  skill: Skill;
  skippedLinks: number;
}

export interface InstallSkillInput {
  tool: AgentTool;
  scope: SkillScope;
  projectPath?: string;
  skill: RemoteSkill;
  collectionId: string;
  overwrite?: boolean;
}
```

(`AgentTool` lives in `src/types/tool.ts`, `Skill`/`SkillScope` in `src/types/skill.ts` — the imports above match.)

Add to `src/types/index.ts`:

```ts
export * from "./collection";
```

- [ ] **Step 3: API client**

Create `src/api/collections.ts`:

```ts
import { invoke } from "@tauri-apps/api/core";
import type {
  CollectionInfo,
  InstallSkillInput,
  InstallResult,
  ListCollectionsResult,
  RemoteSkill,
  SkillManifest,
} from "../types";

/** Invoke wrappers over the collection commands in
 *  src-tauri/src/commands/collections.rs. */
export const collectionsApi = {
  listCollections(): Promise<ListCollectionsResult> {
    return invoke("list_collections");
  },
  browseCollection(id: string): Promise<RemoteSkill[]> {
    return invoke("browse_collection", { id });
  },
  refreshCollection(id: string): Promise<RemoteSkill[]> {
    return invoke("refresh_collection", { id });
  },
  addCollection(repo: string, title?: string): Promise<CollectionInfo> {
    return invoke("add_collection", { repo, title: title ?? null });
  },
  removeCollection(id: string): Promise<void> {
    return invoke("remove_collection", { id });
  },
  installSkill(input: InstallSkillInput): Promise<InstallResult> {
    return invoke("install_skill", {
      tool: input.tool,
      scope: input.scope,
      projectPath: input.projectPath ?? null,
      skill: input.skill,
      collectionId: input.collectionId,
      overwrite: input.overwrite ?? null,
    });
  },
  fetchSkillManifest(skill: RemoteSkill): Promise<SkillManifest> {
    return invoke("fetch_skill_manifest", { skill });
  },
};
```

Update `src/api/index.ts`:

```ts
import { collectionsApi } from "./collections";
import { projectsApi } from "./projects";
import { skillsApi } from "./skills";

/** Single place the frontend talks to the Rust command layer. */
export const api = { ...skillsApi, ...projectsApi, ...collectionsApi };
```

- [ ] **Step 4: Search helper**

Create `src/utils/collectionSearch.ts`:

```ts
import type { RemoteSkill } from "../types";

/** Case-insensitive substring search over name and (when loaded)
 *  description; results sorted by name. Unloaded descriptions simply
 *  don't match — the grid fills them lazily. */
export function searchRemoteSkills(skills: RemoteSkill[], query: string): RemoteSkill[] {
  const q = query.trim().toLowerCase();
  const filtered = q
    ? skills.filter(
        (s) =>
          s.name.toLowerCase().includes(q) ||
          (s.description ?? "").toLowerCase().includes(q),
      )
    : skills;
  return [...filtered].sort((a, b) => a.name.localeCompare(b.name));
}
```

- [ ] **Step 5: Run tests and typecheck**

Run: `npm test && npx tsc --noEmit`
Expected: all tests pass, typecheck clean.

- [ ] **Step 6: Commit**

```bash
git add src/types src/api src/utils
git commit -m "feat(api): add collections client and search helper"
```

---

### Task 8: `useCollections` hook + `BrowseModal` + app wiring

**Files:**
- Create: `src/hooks/useCollections.ts`
- Create: `src/components/modals/BrowseModal.tsx`
- Modify: `src/components/layout/Topbar.tsx`
- Modify: `src/App.tsx`
- Modify: `src/App.css`

**Interfaces:**
- Consumes: `collectionsApi`, `searchRemoteSkills`, `ModalShell`, types from Task 7; `ToolEntry`/`ProjectInfo` from existing types.
- Produces: `useCollections()` returning `{ collections, source, activeId, skills, loading, error, select, refreshActive, add, remove, describe }` where `describe(skill) -> Promise<SkillManifest | null>` fills descriptions with in-memory caching.

- [ ] **Step 1: The hook**

Create `src/hooks/useCollections.ts`:

```ts
import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "../api";
import type {
  CatalogSource,
  CollectionInfo,
  RemoteSkill,
  SkillManifest,
} from "../types";

/** Collection browsing state: the catalog list, the skills of the
 *  active collection, and lazy SKILL.md metadata with in-memory
 *  caching (one fetch per skill per session, deduped in-flight). */
export function useCollections() {
  const [collections, setCollections] = useState<CollectionInfo[]>([]);
  const [source, setSource] = useState<CatalogSource>("bundled");
  const [activeId, setActiveId] = useState<string | null>(null);
  const [skills, setSkills] = useState<RemoteSkill[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const latestRequest = useRef(0);
  const manifests = useRef(new Map<string, SkillManifest>());
  const inFlight = useRef(new Set<string>());

  const loadCollections = useCallback(async () => {
    try {
      const result = await api.listCollections();
      setCollections(result.collections);
      setSource(result.source);
      setActiveId((current) => current ?? result.collections[0]?.id ?? null);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const browse = useCallback(async (id: string, force: boolean) => {
    const request = ++latestRequest.current;
    setLoading(true);
    setError(null);
    try {
      const result = force ? await api.refreshCollection(id) : await api.browseCollection(id);
      if (request !== latestRequest.current) return;
      setSkills(result);
    } catch (e) {
      if (request === latestRequest.current) setError(String(e));
    } finally {
      if (request === latestRequest.current) setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadCollections();
  }, [loadCollections]);

  useEffect(() => {
    if (activeId) browse(activeId, false);
  }, [activeId, browse]);

  const select = useCallback((id: string) => {
    setActiveId(id);
  }, []);

  const refreshActive = useCallback(() => {
    if (activeId) browse(activeId, true);
  }, [activeId, browse]);

  const add = useCallback(
    async (repo: string, title?: string) => {
      const added = await api.addCollection(repo, title);
      await loadCollections();
      setActiveId(added.id);
      return added;
    },
    [loadCollections],
  );

  const remove = useCallback(
    async (id: string) => {
      await api.removeCollection(id);
      if (activeId === id) {
        setSkills([]);
        setActiveId(null);
      }
      await loadCollections();
      setActiveId((current) => current ?? null);
    },
    [activeId, loadCollections],
  );

  /** Lazily fill one skill's description; resolves null while loading
   *  or on failure (the card keeps its name-only rendering). */
  const describe = useCallback(async (skill: RemoteSkill): Promise<SkillManifest | null> => {
    const key = `${skill.owner}/${skill.repo}/${skill.path}`;
    const cached = manifests.current.get(key);
    if (cached) return cached;
    if (inFlight.current.has(key)) return null;
    inFlight.current.add(key);
    try {
      const manifest = await api.fetchSkillManifest(skill);
      manifests.current.set(key, manifest);
      setSkills((current) =>
        current.map((s) =>
          s === skill ? { ...s, description: manifest.description } : s,
        ),
      );
      return manifest;
    } catch {
      return null;
    } finally {
      inFlight.current.delete(key);
    }
  }, []);

  return {
    collections,
    source,
    activeId,
    skills,
    loading,
    error,
    select,
    refreshActive,
    add,
    remove,
    describe,
  };
}
```

- [ ] **Step 2: The modal**

Create `src/components/modals/BrowseModal.tsx`:

```tsx
import { useEffect, useMemo, useState } from "react";
import { api } from "../../api";
import { searchRemoteSkills } from "../../utils/collectionSearch";
import { CloseIcon } from "../ui/icons";
import { ModalShell } from "../ui/ModalShell";
import { useCollections } from "../../hooks/useCollections";
import type {
  AgentTool,
  ProjectInfo,
  RemoteSkill,
  Skill,
  ToolEntry,
} from "../../types";

interface BrowseModalProps {
  toolEntries: ToolEntry[];
  projects: ProjectInfo[];
  /** The project view the modal was opened from, if any. */
  activeProject: ProjectInfo | null;
  defaultTool?: AgentTool;
  onClose: () => void;
  onInstalled: (skill: Skill) => void;
}

function ownTool(entry: ToolEntry): AgentTool | undefined {
  return entry.folders.find((f) => f.role === "own")?.tool ?? entry.folders[0]?.tool;
}

/** Browse GitHub collections and install skills into any managed agent
 *  folder. Descriptions load lazily from each remote SKILL.md. */
export function BrowseModal({
  toolEntries,
  projects,
  activeProject,
  defaultTool,
  onClose,
  onInstalled,
}: BrowseModalProps) {
  const browseState = useCollections();
  const [query, setQuery] = useState("");
  const [addRepo, setAddRepo] = useState("");
  const [addError, setAddError] = useState<string | null>(null);
  const [installing, setInstalling] = useState<RemoteSkill | null>(null);
  const [tool, setTool] = useState<AgentTool>(defaultTool ?? "claude");
  const [scope, setScope] = useState<"user" | "project">(activeProject ? "project" : "user");
  const [projectPath, setProjectPath] = useState<string>(activeProject?.path ?? "");
  const [installError, setInstallError] = useState<string | null>(null);
  const [installedNames, setInstalledNames] = useState<string[]>([]);

  const filtered = useMemo(
    () => searchRemoteSkills(browseState.skills, query),
    [browseState.skills, query],
  );

  // Fill descriptions for the visible, unloaded skills (one pass per
  // skill per session; the hook dedupes and caches).
  useEffect(() => {
    for (const skill of filtered) {
      if (skill.description === null) browseState.describe(skill);
    }
  }, [filtered, browseState.describe]);

  async function install(skill: RemoteSkill) {
    if (!browseState.activeId) return;
    setInstallError(null);
    try {
      const result = await api.installSkill({
        tool,
        scope,
        projectPath: scope === "project" ? projectPath : undefined,
        skill,
        collectionId: browseState.activeId,
      });
      setInstalledNames((names) => [...names, skill.name]);
      setInstalling(null);
      onInstalled(result.skill);
    } catch (e) {
      const message = String(e);
      if (message.includes("already exists")) {
        const overwrite = window.confirm(
          `'${skill.name}' already exists in that folder. Overwrite it?`,
        );
        if (overwrite) {
          try {
            const result = await api.installSkill({
              tool,
              scope,
              projectPath: scope === "project" ? projectPath : undefined,
              skill,
              collectionId: browseState.activeId!,
              overwrite: true,
            });
            setInstalledNames((names) => [...names, skill.name]);
            setInstalling(null);
            onInstalled(result.skill);
            return;
          } catch (retryError) {
            setInstallError(String(retryError));
            return;
          }
        }
        return;
      }
      setInstallError(message);
    }
  }

  async function submitAdd() {
    const repo = addRepo.trim();
    if (!repo) return;
    setAddError(null);
    try {
      await browseState.add(repo);
      setAddRepo("");
    } catch (e) {
      setAddError(String(e));
    }
  }

  return (
    <ModalShell className="browse-modal" onClose={onClose}>
      <div className="modal-header">
        <span className="title">browse collections</span>
        <button className="icon-btn square" onClick={onClose} title="close">
          <CloseIcon />
        </button>
      </div>

      <div className="browse-layout">
        <aside className="collection-pane">
          {browseState.source === "bundled" && (
            <div className="collection-notice">built-in list (catalog unreachable)</div>
          )}
          {browseState.collections.map((collection) => (
            <div key={collection.id} className="collection-row">
              <button
                className={`collection-item ${collection.id === browseState.activeId ? "active" : ""}`}
                onClick={() => browseState.select(collection.id)}
              >
                <span className="collection-title">{collection.title}</span>
                <span className="collection-meta">
                  {collection.repo}
                  {collection.skillCount !== null ? ` · ${collection.skillCount} skills` : ""}
                </span>
              </button>
              {!collection.builtin && (
                <button
                  className="icon-btn square"
                  title="remove collection"
                  onClick={() => browseState.remove(collection.id)}
                >
                  <CloseIcon />
                </button>
              )}
            </div>
          ))}
          <div className="collection-add">
            <input
              className="add-search"
              placeholder="owner/repo"
              value={addRepo}
              spellCheck={false}
              onChange={(e) => setAddRepo(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && submitAdd()}
            />
            <button className="btn" onClick={submitAdd} disabled={!addRepo.trim()}>
              add
            </button>
          </div>
          {addError && <div className="create-error">{addError}</div>}
        </aside>

        <div className="browse-main">
          <div className="browse-toolbar">
            <input
              className="add-search"
              placeholder="search skills..."
              value={query}
              onChange={(e) => setQuery(e.target.value)}
            />
            <button className="btn" onClick={browseState.refreshActive} disabled={browseState.loading}>
              {browseState.loading ? "loading…" : "refresh"}
            </button>
          </div>

          {browseState.error && <div className="create-error">{browseState.error}</div>}

          <div className="skill-grid">
            {filtered.map((skill) => {
              const installed = installedNames.includes(skill.name);
              const isTarget = installing?.name === skill.name;
              return (
                <div key={`${skill.owner}/${skill.repo}/${skill.path}`} className="remote-skill-card">
                  <div className="remote-skill-name">{skill.name}</div>
                  <div className="remote-skill-desc">
                    {skill.description ?? "…"}
                  </div>
                  <div className="remote-skill-meta">{skill.repo}</div>
                  {isTarget ? (
                    <div className="install-row">
                      <select value={tool} onChange={(e) => setTool(e.target.value as AgentTool)}>
                        {toolEntries.map((entry) => {
                          const value = ownTool(entry);
                          return value === undefined ? null : (
                            <option key={entry.id} value={value}>
                              {entry.label}
                            </option>
                          );
                        })}
                      </select>
                      <select
                        value={scope}
                        onChange={(e) => setScope(e.target.value as "user" | "project")}
                      >
                        <option value="user">user</option>
                        <option value="project" disabled={projects.length === 0}>
                          project
                        </option>
                      </select>
                      {scope === "project" && !activeProject && (
                        <select value={projectPath} onChange={(e) => setProjectPath(e.target.value)}>
                          {projects.map((p) => (
                            <option key={p.path} value={p.path}>
                              {p.name}
                            </option>
                          ))}
                        </select>
                      )}
                      <button className="btn" onClick={() => install(skill)}>
                        install
                      </button>
                      <button className="btn" onClick={() => setInstalling(null)}>
                        cancel
                      </button>
                      {installError && <div className="create-error">{installError}</div>}
                    </div>
                  ) : (
                    <button
                      className="btn"
                      onClick={() => setInstalling(skill)}
                      disabled={installed}
                    >
                      {installed ? "installed ✓" : "add"}
                    </button>
                  )}
                </div>
              );
            })}
            {!browseState.loading && filtered.length === 0 && !browseState.error && (
              <div className="empty-state">no skills in this collection.</div>
            )}
          </div>
        </div>
      </div>
    </ModalShell>
  );
}
```

- [ ] **Step 3: Topbar button**

In `src/components/layout/Topbar.tsx`, add to the props interface:

```ts
  /** Opens the collections browse flow. */
  onBrowse?: () => void;
```

Destructure it in the component signature, and in `topbar-actions` render before the "new skill" button:

```tsx
        {onBrowse && (
          <button className="btn" onClick={onBrowse} title="browse and install skills from GitHub collections">
            browse
          </button>
        )}
```

- [ ] **Step 4: Wire into App**

In `src/App.tsx`: import the modal,

```ts
import { BrowseModal } from "./components/modals/BrowseModal";
```

add state next to `creatingSkill`:

```ts
  const [browsing, setBrowsing] = useState(false);
```

pass to Topbar (next to `onNewSkill`):

```tsx
          onBrowse={() => setBrowsing(true)}
```

and render beside the other modals (after the `creatingSkill` block):

```tsx
      {browsing && (
        <BrowseModal
          toolEntries={global.toolEntries}
          projects={projects.projects}
          activeProject={activeProject}
          defaultTool={
            activeTool
              ? (activeTool.folders.find((f) => f.role === "own")?.tool ??
                activeTool.folders[0]?.tool) as AgentTool | undefined
              : undefined
          }
          onClose={() => setBrowsing(false)}
          onInstalled={async (skill) => {
            await global.refresh();
            if (skill.scope === "project") {
              await projects.refresh();
              projectView.reload();
            }
          }}
        />
      )}
```

- [ ] **Step 5: Styles**

Append to `src/App.css`:

```css
/* ------------------------------------------------------------------ */
/* Browse collections modal                                            */
/* ------------------------------------------------------------------ */

.browse-modal {
  width: min(880px, 92vw);
}

.browse-layout {
  display: flex;
  gap: 16px;
  min-height: 420px;
  max-height: min(64vh, 560px);
}

.collection-pane {
  display: flex;
  flex-direction: column;
  gap: 6px;
  width: 220px;
  flex-shrink: 0;
  overflow-y: auto;
  padding-right: 8px;
}

.collection-notice {
  font-size: 11px;
  opacity: 0.7;
  margin-bottom: 4px;
}

.collection-row {
  display: flex;
  align-items: stretch;
  gap: 4px;
}

.collection-item {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 2px;
  text-align: left;
  padding: 8px;
  border-radius: 8px;
  border: 1px solid transparent;
  background: none;
  color: inherit;
  cursor: pointer;
}

.collection-item:hover {
  border-color: var(--border);
}

.collection-item.active {
  border-color: var(--border);
  background: rgba(128, 128, 128, 0.12);
}

.collection-title {
  font-size: 13px;
  font-weight: 600;
}

.collection-meta {
  font-size: 11px;
  opacity: 0.65;
}

.collection-add {
  display: flex;
  gap: 6px;
  margin-top: auto;
  padding-top: 10px;
}

.browse-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 10px;
  min-width: 0;
}

.browse-toolbar {
  display: flex;
  gap: 8px;
}

.browse-toolbar .add-search {
  flex: 1;
}

.skill-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: 10px;
  overflow-y: auto;
  align-content: start;
}

.remote-skill-card {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 12px;
  border: 1px solid var(--border);
  border-radius: 10px;
}

.remote-skill-name {
  font-size: 13px;
  font-weight: 600;
}

.remote-skill-desc {
  font-size: 12px;
  opacity: 0.75;
  min-height: 32px;
  overflow: hidden;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
}

.remote-skill-meta {
  font-size: 11px;
  opacity: 0.6;
}

.install-row {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  align-items: center;
}
```

- [ ] **Step 6: Typecheck and smoke-test**

Run: `npx tsc --noEmit && npm test`
Expected: clean and green.

Run: `npm run tauri dev`
Manual checks: "browse" button opens the modal; the three built-in collections appear (bundled fallback until `collections.json` exists upstream); clicking one lists its skills; descriptions fill in; "add" → tool/scope picker → install writes the folder (verify in `~/.claude/skills` or a project's `.claude/skills`) with `.collection-source.json` inside; installing a duplicate prompts to overwrite; adding `anthropics/skills` by URL reports "already"; removing a built-in errors visibly. Close the dev app.

- [ ] **Step 7: Commit**

```bash
git add src/App.tsx src/App.css src/components src/hooks
git commit -m "feat(modals): add BrowseModal for installing remote skills"
```

---

### Task 9: Docs + full gates

**Files:**
- Modify: `AGENTS.md` (structure section)

**Interfaces:**
- Consumes: none.
- Produces: documentation only.

- [ ] **Step 1: Update AGENTS.md structure**

In the `src-tauri/src/` block of the structure tree, after the `commands/` entry, add:

```
  collections/         GitHub collections: github.rs (HTTP + parsing), catalog.rs
                        (tree → skills), manifest.rs (public catalog +
                        bundled fallback), install.rs (download into managed
                        roots + provenance), store.rs (user collections, cache)
```

and extend the `commands/` line's parenthetical to mention `collections.rs`.

- [ ] **Step 2: Full gates**

Run, in order:
- `npx tsc --noEmit` — clean
- `npm test` — green
- `cd src-tauri && cargo fmt` — no diff
- `cd src-tauri && cargo clippy --all-targets` — warning-free
- `cd src-tauri && cargo test` — green

- [ ] **Step 3: Commit**

```bash
git add AGENTS.md
git commit -m "docs(agents): describe the collections module"
```

---

## Follow-up (outside this repo)

The managed catalog needs `collections.json` at the root of `abubakarsiddik31/claude-skills-collection`, seeded with the same three entries as `fallback.json`. Until it exists, every install uses the bundled fallback. Open a PR against that repo when ready — content:

```json
{
  "version": 1,
  "collections": [
    { "id": "anthropics-skills", "title": "Anthropic Skills", "repo": "anthropics/skills" },
    { "id": "superpowers", "title": "Superpowers", "repo": "obra/superpowers" },
    { "id": "matt-skills", "title": "Matt's Skills", "repo": "mattpocock/skills" }
  ]
}
```
