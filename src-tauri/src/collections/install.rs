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
/// no dot components and no `:` in any component — a drive letter
/// (`C:/evil`) would make `PathBuf::join` produce an absolute path on
/// Windows, escaping the managed roots. GitHub tree paths use `/`
/// separators only.
pub fn safe_relative(rel: &str) -> bool {
    !rel.is_empty()
        && !rel.starts_with('/')
        && !rel.contains('\\')
        && rel
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != ".." && !part.contains(':'))
}

/// The blobs of one skill folder, relative to it. Symlinks (mode
/// 120000) and submodules (160000) are skipped and counted — they are
/// never materialized on disk. `folder == ""` means the whole repo is
/// the skill.
pub fn files_for_skill(
    tree: &super::github::TreeResponse,
    folder: &str,
) -> Result<(Vec<RemoteFile>, usize), String> {
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
/// temp dir and renamed in: the existing skill folder is removed only
/// after the replacement has been fully downloaded, so a failed
/// download never destroys the previously installed skill (which may
/// hold local user edits) and leaves nothing behind.
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
    let existing = target.exists();
    if existing {
        if !overwrite {
            return Err(format!(
                "a skill named '{name}' already exists in this folder"
            ));
        }
        if !crate::commands::validate_manifest_at(&target.join("SKILL.md"), roots) {
            return Err(format!(
                "'{name}' exists but is not a skill this app manages; remove it manually first"
            ));
        }
    }

    fs::create_dir_all(root).map_err(|e| format!("cannot create target folder: {e}"))?;
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let tmp = root.join(format!(".{name}.tmp-{}-{nanos}", std::process::id()));

    let build = || -> Result<(), String> {
        for file in files {
            // defense in depth: every written path is re-checked here,
            // at the boundary that touches the filesystem
            if !safe_relative(&file.relative_path) {
                return Err(format!(
                    "remote path '{}' is not safe to write",
                    file.relative_path
                ));
            }
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
    if existing {
        if let Err(e) = fs::remove_dir_all(&target) {
            let _ = fs::remove_dir_all(&tmp);
            return Err(format!("cannot remove existing skill: {e}"));
        }
    }
    if let Err(e) = fs::rename(&tmp, &target) {
        let _ = fs::remove_dir_all(&tmp);
        return Err(format!("cannot finalize install: {e}"));
    }
    Ok(target.join("SKILL.md"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::github::parse_tree;
    use crate::collections::github::testutil::FakeGithubHttp;
    use std::fs;
    use std::path::PathBuf;

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
        http.mount(
            "blobs/sha-skill",
            r#"{"content": "LS1tLQpuYW1lOiBwZGY=", "encoding": "base64"}"#,
        );
        http.mount(
            "blobs/sha-script",
            r#"{"content": "cHJpbnQoJ2hpJyk=", "encoding": "base64"}"#,
        );
        http
    }

    fn dirs(tag: &str) -> (PathBuf, PathBuf) {
        let base = std::env::temp_dir().join(format!(
            "skill-manager-install-{tag}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("managed")).unwrap();
        (base.clone(), base.join("managed"))
    }

    #[test]
    fn safe_relative_rejects_traversal() {
        assert!(safe_relative("SKILL.md"));
        assert!(safe_relative("scripts/run.py"));
        for bad in [
            "",
            "/abs",
            "../up",
            "a/../../b",
            "a//b",
            "a/./b",
            "..",
            "a\\b",
            "a/../b",
            "C:/evil",
            "c:",
            "a/b:c",
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

        let err = install_skill_files(
            &http,
            &roots,
            &unmanaged,
            "pdf",
            &files,
            &provenance(),
            false,
        )
        .unwrap_err();
        assert!(err.contains("not managed"), "unexpected error: {err}");

        let manifest =
            install_skill_files(&http, &roots, &managed, "pdf", &files, &provenance(), false)
                .unwrap();
        assert!(manifest.is_file());
        assert!(managed.join("pdf/scripts/run.py").is_file());
        let prov: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(managed.join("pdf/").join(PROVENANCE_FILE)).unwrap(),
        )
        .unwrap();
        assert_eq!(prov["treeSha"], "tree-sha");
        assert_eq!(prov["collectionId"], "anthropics-skills");
        // no temp dirs left behind
        let leftovers: Vec<_> = fs::read_dir(&managed)
            .unwrap()
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

        let err = install_skill_files(&http, &roots, &managed, "pdf", &files, &provenance(), false)
            .unwrap_err();
        assert!(err.contains("already exists"), "unexpected error: {err}");

        // overwrite replaces the skill (same content → same tree sha)
        install_skill_files(&http, &roots, &managed, "pdf", &files, &provenance(), true).unwrap();

        // overwrite refuses to delete something that is not a managed skill shape
        fs::create_dir_all(managed.join("not-a-skill")).unwrap();
        fs::write(managed.join("not-a-skill/notes.txt"), "x").unwrap();
        let err = install_skill_files(
            &http,
            &roots,
            &managed,
            "not-a-skill",
            &files,
            &provenance(),
            true,
        )
        .unwrap_err();
        assert!(
            err.contains("not a skill this app manages"),
            "unexpected error: {err}"
        );
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
            assert!(install_skill_files(
                &http,
                &roots,
                &managed,
                bad,
                &files,
                &provenance(),
                false
            )
            .is_err());
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
        let err = install_skill_files(&http, &roots, &managed, "pdf", &files, &provenance(), false)
            .unwrap_err();
        assert!(
            err.contains("no fixture") || err.contains("GitHub"),
            "unexpected error: {err}"
        );
        assert!(!managed.join("pdf").exists());

        let _ = fs::remove_dir_all(managed.parent().unwrap());
    }

    #[test]
    fn overwrite_failure_preserves_the_existing_skill() {
        let (_base, managed) = dirs("preserve");
        let roots = vec![managed.clone()];
        let tree = parse_tree(TREE_JSON).unwrap();
        let (files, _) = files_for_skill(&tree, "skills/pdf").unwrap();
        let http = http_with_blobs();
        install_skill_files(&http, &roots, &managed, "pdf", &files, &provenance(), false).unwrap();

        // mark the installed skill as holding unmistakable local edits
        fs::write(managed.join("pdf/SKILL.md"), "original-local-edits").unwrap();

        // no fixtures mounted → every blob fetch fails mid-overwrite
        let failing = FakeGithubHttp::new();
        let err = install_skill_files(
            &failing,
            &roots,
            &managed,
            "pdf",
            &files,
            &provenance(),
            true,
        )
        .unwrap_err();
        assert!(
            err.contains("no fixture") || err.contains("GitHub"),
            "unexpected error: {err}"
        );

        // the existing skill survived the failed overwrite untouched
        let skill_md = managed.join("pdf/SKILL.md");
        assert!(skill_md.is_file());
        assert_eq!(
            fs::read_to_string(&skill_md).unwrap(),
            "original-local-edits"
        );
        // no temp dirs left behind
        let leftovers: Vec<_> = fs::read_dir(&managed)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp-"))
            .collect();
        assert!(leftovers.is_empty());

        let _ = fs::remove_dir_all(managed.parent().unwrap());
    }
}
