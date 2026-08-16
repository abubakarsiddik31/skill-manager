//! Project auto-detection: find the folders a user actually works in so
//! the app can suggest them instead of making everyone browse manually.
//!
//! Sources, in order of confidence:
//! - Claude Code's project history (`~/.claude.json`) — real agent usage,
//!   with last-active time taken from the session folder it writes to
//! - Cursor / VS Code global state — recently opened workspaces
//! - a bounded scan of common dev roots for `.git` folders
//!
//! Everything is best-effort: unreadable or missing sources are skipped,
//! never fatal.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedProject {
    pub path: String,
    pub name: String,
    /// Unix seconds of the strongest activity signal found; 0 = unknown.
    pub last_active: u64,
    /// Agent skills are deliberately not counted during discovery: reading
    /// every candidate would trigger access requests for protected folders.
    pub skill_count: Option<usize>,
    pub sources: Vec<String>,
}

#[derive(Default)]
struct Candidate {
    last_active: u64,
    sources: BTreeSet<&'static str>,
}

/// Folders scanned for git repos, relative to the home directory. Kept
/// deliberately small — detection should stay fast, not enumerate disk.
const COMMON_ROOTS: &[&str] = &[
    "Projects", "projects", "dev", "src", "code", "repos", "github", "work",
];
const GIT_SCAN_DEPTH: usize = 2;
const GIT_SCAN_BUDGET: usize = 500;
const MAX_RESULTS: usize = 50;

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn mtime(path: &Path) -> u64 {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

type Candidates = HashMap<PathBuf, Candidate>;

fn record(out: &mut Candidates, path: PathBuf, source: &'static str, last_active: u64) {
    let entry = out.entry(path).or_default();
    entry.sources.insert(source);
    entry.last_active = entry.last_active.max(last_active);
}

fn collect_claude(out: &mut Candidates) {
    let Some(home) = home() else { return };
    let Ok(raw) = fs::read_to_string(home.join(".claude.json")) else {
        return;
    };
    let Ok(value) = serde_json::from_str::<Value>(&raw) else {
        return;
    };
    let Some(projects) = value.get("projects").and_then(|p| p.as_object()) else {
        return;
    };
    // Claude Code stores session transcripts under ~/.claude/projects/,
    // in folders named after the project path with every non-alphanumeric
    // character replaced by '-'. That folder's mtime is the best
    // last-active signal available without parsing the transcripts.
    let sessions_root = home.join(".claude").join("projects");
    for path in projects.keys() {
        let munged: String = path
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        record(
            out,
            PathBuf::from(path),
            "claude",
            mtime(&sessions_root.join(munged)),
        );
    }
}

/// Reads a `storage.json` global-state file and keeps every `file://`
/// URI it contains — editors move this data between keys and formats
/// (json vs sqlite), but the URIs themselves are stable.
fn collect_editor(home: &Path, app_dir: &str, source: &'static str, out: &mut Candidates) {
    let candidates = [
        home.join("Library/Application Support"),
        home.join("AppData/Roaming"),
        home.join(".config"),
    ];
    let storage = candidates
        .iter()
        .map(|base| base.join(app_dir).join("User/globalStorage/storage.json"))
        .find_map(|p| fs::read_to_string(p).ok());
    let Some(raw) = storage else { return };
    let Ok(value) = serde_json::from_str::<Value>(&raw) else {
        return;
    };

    let mut uris = Vec::new();
    collect_file_uris(&value, &mut uris);
    for uri in uris {
        let Some(path) = file_uri_to_path(&uri) else {
            continue;
        };
        record(out, path, source, 0);
    }
}

fn collect_file_uris(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(s) if s.starts_with("file://") => out.push(s.clone()),
        Value::Array(items) => items.iter().for_each(|v| collect_file_uris(v, out)),
        Value::Object(map) => map.values().for_each(|v| collect_file_uris(v, out)),
        _ => {}
    }
}

/// A `file://` URI is only a local path when its host is empty or
/// `localhost` — anything else names a network share, which we don't
/// want to suggest as a project folder.
fn file_uri_to_path(uri: &str) -> Option<PathBuf> {
    let rest = &uri["file://".len()..];
    let rest = match rest.find('/') {
        Some(0) => rest,
        Some(at) if rest[..at].eq_ignore_ascii_case("localhost") => &rest[at..],
        _ => return None,
    };
    // percent-decode into raw bytes, then interpret as utf-8
    let bytes = rest.as_bytes();
    let mut decoded: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(&rest[i + 1..i + 3], 16) {
                decoded.push(byte);
                i += 3;
                continue;
            }
        }
        decoded.push(bytes[i]);
        i += 1;
    }
    let mut path = String::from_utf8_lossy(&decoded).into_owned();
    // windows URIs look like file:///C:/... — drop the slash before the drive
    let bytes = path.as_bytes();
    if bytes.len() > 3 && bytes[0] == b'/' && bytes[1].is_ascii_alphabetic() && bytes[2] == b':' {
        path.remove(0);
    }
    while path.ends_with('/') {
        path.pop();
    }
    Some(PathBuf::from(path))
}

fn collect_git(out: &mut Candidates) {
    let Some(home) = home() else { return };
    for root in COMMON_ROOTS {
        let mut budget = GIT_SCAN_BUDGET;
        scan_for_git(&home.join(root), 0, &mut budget, out);
    }
}

fn scan_for_git(dir: &Path, depth: usize, budget: &mut usize, out: &mut Candidates) {
    if depth > GIT_SCAN_DEPTH || *budget == 0 {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if *budget == 0 {
            return;
        }
        *budget -= 1;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') || name == "node_modules" || name == "Library" {
            continue;
        }
        let git = path.join(".git");
        if git.exists() {
            // commits and branch switches touch .git and HEAD
            record(out, path, "git", mtime(&git).max(mtime(&git.join("HEAD"))));
            continue; // don't descend into repos
        }
        scan_for_git(&path, depth + 1, budget, out);
    }
}

/// True for folders that are containers, not projects: the home folder,
/// anything at or above it, and its immediate well-known children (our
/// scan roots plus the standard macOS user folders). Claude Code's
/// history contains launches in these, but suggesting them as projects
/// is noise.
fn is_container(path: &Path, home: &Path) -> bool {
    if path.components().count() <= home.components().count() {
        return true;
    }
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return true;
    };
    if path.parent() == Some(home) {
        const USER_DIRS: &[&str] = &[
            "Desktop",
            "Documents",
            "Downloads",
            "Movies",
            "Music",
            "Pictures",
            "Public",
        ];
        return COMMON_ROOTS.contains(&name) || USER_DIRS.contains(&name);
    }
    false
}

pub fn detect(exclude: &[String]) -> Vec<DetectedProject> {
    let Some(home) = home() else {
        return Vec::new();
    };
    let mut candidates: Candidates = HashMap::new();
    collect_claude(&mut candidates);
    collect_editor(&home, "Cursor", "cursor", &mut candidates);
    collect_editor(&home, "Code", "vscode", &mut candidates);
    collect_git(&mut candidates);

    let normalize = |p: &str| p.trim_end_matches('/').to_string();
    let excluded: Vec<String> = exclude.iter().map(|p| normalize(p)).collect();

    let mut list: Vec<DetectedProject> = candidates
        .into_iter()
        .filter(|(path, _)| !is_container(path, &home))
        .filter(|(path, _)| !excluded.contains(&normalize(&path.to_string_lossy())))
        .map(|(path, candidate)| {
            // A project path recovered from editor or agent history may live
            // in Documents or Desktop. Do not touch it here: macOS may ask
            // for access, and the user has not chosen that project yet.
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.to_string_lossy().into_owned());
            DetectedProject {
                path: path.to_string_lossy().into_owned(),
                name,
                last_active: candidate.last_active,
                skill_count: None,
                sources: candidate.sources.into_iter().map(str::to_owned).collect(),
            }
        })
        .collect();

    list.sort_by(|a, b| {
        b.last_active
            .cmp(&a.last_active)
            .then_with(|| a.path.cmp(&b.path))
    });
    list.truncate(MAX_RESULTS);
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_file_uris() {
        assert_eq!(
            file_uri_to_path("file:///Users/foo/My%20Project"),
            Some(PathBuf::from("/Users/foo/My Project"))
        );
        assert_eq!(
            file_uri_to_path("file:///C%3A/Users/foo/app"),
            Some(PathBuf::from("C:/Users/foo/app"))
        );
        assert_eq!(
            file_uri_to_path("file:///home/foo/app/"),
            Some(PathBuf::from("/home/foo/app"))
        );
        assert_eq!(
            file_uri_to_path("file://localhost/home/foo"),
            Some(PathBuf::from("/home/foo"))
        );
        // network shares are not local paths
        assert_eq!(file_uri_to_path("file://server/share/doc"), None);
        assert_eq!(file_uri_to_path("file://example.com/a"), None);
    }

    #[test]
    fn collects_uris_from_nested_json() {
        let value = serde_json::json!({
            "known": ["file:///a/one", "https://example.com"],
            "nested": { "deep": "file:///a/two" }
        });
        let mut uris = Vec::new();
        collect_file_uris(&value, &mut uris);
        uris.sort();
        assert_eq!(uris, vec!["file:///a/one", "file:///a/two"]);
    }

    #[test]
    fn containers_are_not_projects() {
        let home = Path::new("/Users/foo");
        // home itself and everything at or above it
        for path in ["/Users/foo", "/Users", "/"] {
            assert!(is_container(Path::new(path), home), "{path}");
        }
        // well-known children of home
        for path in [
            "/Users/foo/Desktop",
            "/Users/foo/Documents",
            "/Users/foo/Downloads",
            "/Users/foo/projects",
        ] {
            assert!(is_container(Path::new(path), home), "{path}");
        }
        // real project folders pass
        for path in [
            "/Users/foo/Desktop/app",
            "/Users/foo/projects/app",
            "/Users/foo/app",
        ] {
            assert!(!is_container(Path::new(path), home), "{path}");
        }
    }

    #[test]
    fn background_git_scan_never_walks_privacy_sensitive_user_folders() {
        assert!(!COMMON_ROOTS.contains(&"Desktop"));
        assert!(!COMMON_ROOTS.contains(&"Documents"));
    }
}
