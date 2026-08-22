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
        return Err(format!(
            "'{repo}' is not a valid owner/repo repository slug"
        ));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir(tag: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("skill-manager-store-{tag}-{}", std::process::id()));
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
        for bad in ["nodir", "a/b/c", "../escape", "a/", "/b", "a b/c", ".a/b"] {
            assert!(
                add_user_collection(&dir, bad, None).is_err(),
                "{bad:?} should be rejected"
            );
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
                tree: crate::collections::github::parse_tree(r#"{"sha": "x", "tree": []}"#)
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
