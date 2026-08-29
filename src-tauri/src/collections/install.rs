use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Where install provenance lives inside every installed skill folder.
pub const PROVENANCE_FILE: &str = ".collection-source.json";

/// One file of a skill, extracted from the repository tarball.
#[derive(Debug, Clone)]
pub struct SkillFile {
    /// Path relative to the skill folder — checked for safety before any write.
    pub relative_path: String,
    pub bytes: Vec<u8>,
    /// Unix permission bits (`0o755` etc.) preserved from the archive.
    pub mode: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Provenance {
    pub owner: String,
    pub repo: String,
    pub path: String,
    /// The git ref the tarball was served for — a branch name, or
    /// `HEAD` when the app had no cached branch for the repo.
    pub branch: String,
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

/// Extract one skill folder's files from a repository tarball. Every
/// entry sits under the archive's `<repo>-<ref>/` root, which is
/// stripped; symlinks and other non-regular entries are skipped and
/// counted — they are never materialized on disk. `folder == ""` means
/// the whole repo is the skill. One tarball download replaces one blob
/// request per file, so installing never spends REST API quota.
pub fn files_from_tarball(tarball: &[u8], folder: &str) -> Result<(Vec<SkillFile>, usize), String> {
    let prefix = if folder.is_empty() {
        String::new()
    } else {
        format!("{folder}/")
    };
    let mut archive = tar::Archive::new(GzDecoder::new(tarball));
    let entries = archive
        .entries()
        .map_err(|e| format!("cannot open repository archive: {e}"))?;
    let mut files = Vec::new();
    let mut skipped = 0;
    let mut has_manifest = false;
    for entry in entries {
        let mut entry = entry.map_err(|e| format!("cannot read repository archive: {e}"))?;
        let kind = entry.header().entry_type();
        let Some(path) = entry
            .path()
            .ok()
            .and_then(|p| p.to_str().map(str::to_string))
        else {
            skipped += 1;
            continue;
        };
        // every entry sits under the archive's `<repo>-<ref>/` root
        let Some((_, rest)) = path.split_once('/') else {
            continue;
        };
        let Some(rel) = rest.strip_prefix(prefix.as_str()) else {
            continue;
        };
        if kind == tar::EntryType::Directory {
            continue;
        }
        if kind != tar::EntryType::Regular {
            skipped += 1;
            continue;
        }
        if !safe_relative(rel) {
            return Err(format!("archive path '{rel}' is not safe to write"));
        }
        let mut bytes = Vec::new();
        entry
            .read_to_end(&mut bytes)
            .map_err(|e| format!("cannot read '{rel}' from archive: {e}"))?;
        if rel == "SKILL.md" {
            has_manifest = true;
        }
        let mode = entry.header().mode().unwrap_or(0o644) & 0o777;
        files.push(SkillFile {
            relative_path: rel.to_string(),
            bytes,
            mode,
        });
    }
    if !has_manifest {
        return Err(format!(
            "'{folder}' has no SKILL.md in the repository archive"
        ));
    }
    Ok((files, skipped))
}

/// Write `files` into `<root>/<name>/` and return the manifest path.
/// Mirrors create_skill's security model: `root` must be exactly one
/// of the managed `roots`, `name` passes the folder-name policy,
/// collisions are explicit, and overwriting only removes folders whose
/// manifest passes `validate_manifest_at`. Everything is built in a
/// temp dir and renamed in: the existing skill folder is removed only
/// after the replacement has been fully written, so a failure never
/// destroys the previously installed skill (which may hold local user
/// edits) and leaves nothing behind.
pub fn install_skill_files(
    roots: &[PathBuf],
    root: &Path,
    name: &str,
    files: &[SkillFile],
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
                    "archive path '{}' is not safe to write",
                    file.relative_path
                ));
            }
            let dest = tmp.join(&file.relative_path);
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent).map_err(|e| format!("cannot create folders: {e}"))?;
            }
            fs::write(&dest, &file.bytes).map_err(|e| format!("cannot write file: {e}"))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&dest, fs::Permissions::from_mode(file.mode));
            }
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
    use std::fs;
    use std::path::PathBuf;

    struct Fixture {
        path: &'static str,
        contents: &'static [u8],
        mode: u32,
        symlink: bool,
    }

    fn file(path: &'static str, contents: &'static [u8]) -> Fixture {
        Fixture {
            path,
            contents,
            mode: 0o644,
            symlink: false,
        }
    }

    fn exec(path: &'static str, contents: &'static [u8]) -> Fixture {
        Fixture {
            path,
            contents,
            mode: 0o755,
            symlink: false,
        }
    }

    fn link(path: &'static str) -> Fixture {
        Fixture {
            path,
            contents: b"",
            mode: 0o777,
            symlink: true,
        }
    }

    /// Build a gzipped tarball shaped like a codeload download: every
    /// path prefixed with the archive's `<repo>-<ref>/` root.
    fn tarball(entries: &[Fixture]) -> Vec<u8> {
        let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        {
            let mut archive = tar::Builder::new(&mut gzip);
            for entry in entries {
                let mut header = tar::Header::new_gnu();
                header.set_size(entry.contents.len() as u64);
                header.set_mode(entry.mode);
                if entry.symlink {
                    header.set_entry_type(tar::EntryType::Symlink);
                    header.set_link_name("/etc/passwd").unwrap();
                }
                header.set_cksum();
                archive
                    .append_data(&mut header, entry.path, entry.contents)
                    .unwrap();
            }
            archive.into_inner().unwrap();
        }
        gzip.finish().unwrap()
    }

    fn repo_tarball() -> Vec<u8> {
        tarball(&[
            file(
                "skills-main/skills/pdf/SKILL.md",
                b"---\nname: pdf\n---\nskill body",
            ),
            file("skills-main/skills/pdf/LICENSE.txt", b"mit"),
            file("skills-main/skills/pdf/scripts/run.py", b"print('hi')"),
            exec("skills-main/skills/pdf/scripts/tool.sh", b"#!/bin/sh\n"),
            link("skills-main/skills/pdf/link"),
            file("skills-main/other/README.md", b"readme"),
        ])
    }

    fn pdf_files() -> Vec<SkillFile> {
        files_from_tarball(&repo_tarball(), "skills/pdf").unwrap().0
    }

    fn provenance() -> Provenance {
        Provenance {
            owner: "anthropics".into(),
            repo: "skills".into(),
            path: "skills/pdf".into(),
            branch: "main".into(),
            installed_at: 1_000,
            collection_id: "anthropics-skills".into(),
        }
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
    #[ignore = "hits the real codeload endpoint; run explicitly with --ignored"]
    fn real_tarball_extracts_a_known_skill() {
        use crate::collections::github::GithubHttp as _;
        let tarball = crate::collections::github::UreqGithubHttp
            .fetch_tarball("anthropics", "skills", "HEAD")
            .unwrap();
        let (files, skipped) = files_from_tarball(&tarball, "skills/pdf").unwrap();
        assert!(files.iter().any(|f| f.relative_path == "SKILL.md"));
        assert!(files.len() > 5, "pdf skill should carry its scripts");
        assert!(skipped == 0, "unexpected non-regular entries: {skipped}");
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
    fn files_from_tarball_selects_folder_files_and_skips_links() {
        let (files, skipped) = files_from_tarball(&repo_tarball(), "skills/pdf").unwrap();
        assert_eq!(skipped, 1); // the symlink entry
        assert_eq!(files.len(), 4);
        assert!(files.iter().any(|f| f.relative_path == "SKILL.md"));
        assert!(files.iter().any(|f| f.relative_path == "scripts/run.py"));
        let manifest = files
            .iter()
            .find(|f| f.relative_path == "SKILL.md")
            .unwrap();
        assert!(String::from_utf8_lossy(&manifest.bytes).contains("skill body"));
        // the executable bit survives the archive round trip
        let tool = files
            .iter()
            .find(|f| f.relative_path == "scripts/tool.sh")
            .unwrap();
        assert_eq!(tool.mode & 0o111, 0o111);
    }

    #[test]
    fn files_from_tarball_requires_a_manifest() {
        let err = files_from_tarball(&repo_tarball(), "other").unwrap_err();
        assert!(err.contains("no SKILL.md"), "unexpected error: {err}");
    }

    #[test]
    fn files_from_tarball_root_folder_takes_the_whole_repo() {
        let whole_repo = tarball(&[
            file("one-skill-main/SKILL.md", b"skill"),
            file("one-skill-main/scripts/run.py", b"run"),
        ]);
        let (files, _) = files_from_tarball(&whole_repo, "").unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|f| f.relative_path == "SKILL.md"));
    }

    #[test]
    fn files_from_tarball_rejects_unsafe_archive_paths() {
        // tar::Builder refuses to even create paths containing "..", so
        // the hostile entry's name is poked into the raw header — the
        // extractor must not trust archive paths any more than tree paths.
        let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        {
            let mut archive = tar::Builder::new(&mut gzip);
            let mut ok = tar::Header::new_gnu();
            ok.set_size(2);
            ok.set_mode(0o644);
            ok.set_cksum();
            archive
                .append_data(&mut ok, "repo-main/SKILL.md", &b"ok"[..])
                .unwrap();

            let mut evil = tar::Header::new_gnu();
            evil.set_size(4);
            evil.set_mode(0o644);
            {
                let raw = evil.as_old_mut();
                let name = b"repo-main/skills/pdf/../../escape.sh";
                raw.name[..name.len()].copy_from_slice(name);
            }
            evil.set_cksum();
            archive.append(&evil, b"nope".as_slice()).unwrap();
            archive.into_inner().unwrap();
        }
        let evil = gzip.finish().unwrap();
        let err = files_from_tarball(&evil, "skills/pdf").unwrap_err();
        assert!(err.contains("not safe to write"), "unexpected error: {err}");
    }

    #[test]
    fn files_from_tarball_rejects_a_corrupt_archive() {
        let err = files_from_tarball(b"this is not a tarball", "skills/pdf").unwrap_err();
        assert!(err.contains("archive"), "unexpected error: {err}");
    }

    #[test]
    fn install_writes_files_provenance_and_rejects_unmanaged_roots() {
        let (_base, managed) = dirs("happy");
        let unmanaged = managed.parent().unwrap().join("unmanaged");
        let roots = vec![managed.clone()];
        let files = pdf_files();

        let err = install_skill_files(&roots, &unmanaged, "pdf", &files, &provenance(), false)
            .unwrap_err();
        assert!(err.contains("not managed"), "unexpected error: {err}");

        let manifest =
            install_skill_files(&roots, &managed, "pdf", &files, &provenance(), false).unwrap();
        assert!(manifest.is_file());
        assert!(managed.join("pdf/scripts/run.py").is_file());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(managed.join("pdf/scripts/tool.sh"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o111, 0o111, "executable bit should be preserved");
        }
        let prov: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(managed.join("pdf/").join(PROVENANCE_FILE)).unwrap(),
        )
        .unwrap();
        assert_eq!(prov["branch"], "main");
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
        let files = pdf_files();
        install_skill_files(&roots, &managed, "pdf", &files, &provenance(), false).unwrap();

        let err =
            install_skill_files(&roots, &managed, "pdf", &files, &provenance(), false).unwrap_err();
        assert!(err.contains("already exists"), "unexpected error: {err}");

        install_skill_files(&roots, &managed, "pdf", &files, &provenance(), true).unwrap();

        // overwrite refuses to delete something that is not a managed skill shape
        fs::create_dir_all(managed.join("not-a-skill")).unwrap();
        fs::write(managed.join("not-a-skill/notes.txt"), "x").unwrap();
        let err = install_skill_files(&roots, &managed, "not-a-skill", &files, &provenance(), true)
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
        let files = pdf_files();
        for bad in ["../escape", ".disabled", "a b"] {
            assert!(
                install_skill_files(&roots, &managed, bad, &files, &provenance(), false).is_err()
            );
        }

        let _ = fs::remove_dir_all(managed.parent().unwrap());
    }

    #[test]
    fn unsafe_files_fail_at_the_write_boundary_and_leave_nothing_behind() {
        let (_base, managed) = dirs("boundary");
        let roots = vec![managed.clone()];
        let mut files = pdf_files();
        files.push(SkillFile {
            relative_path: "../escape".into(),
            bytes: b"nope".to_vec(),
            mode: 0o644,
        });
        let err =
            install_skill_files(&roots, &managed, "pdf", &files, &provenance(), false).unwrap_err();
        assert!(err.contains("not safe to write"), "unexpected error: {err}");
        assert!(!managed.join("pdf").exists());

        let _ = fs::remove_dir_all(managed.parent().unwrap());
    }

    #[test]
    fn overwrite_failure_preserves_the_existing_skill() {
        let (_base, managed) = dirs("preserve");
        let roots = vec![managed.clone()];
        let files = pdf_files();
        install_skill_files(&roots, &managed, "pdf", &files, &provenance(), false).unwrap();

        // mark the installed skill as holding unmistakable local edits
        fs::write(managed.join("pdf/SKILL.md"), "original-local-edits").unwrap();

        // an unsafe file fails the build before anything is replaced
        let mut poisoned = pdf_files();
        poisoned.push(SkillFile {
            relative_path: "a/../../escape".into(),
            bytes: b"nope".to_vec(),
            mode: 0o644,
        });
        let err = install_skill_files(&roots, &managed, "pdf", &poisoned, &provenance(), true)
            .unwrap_err();
        assert!(err.contains("not safe to write"), "unexpected error: {err}");

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
