use serde::{Deserialize, Serialize};

use super::github::GithubHttp;
use super::store::{CollectionsCache, UserCollection};
use super::CollectionInfo;

pub const CATALOG_URL: &str = "https://raw.githubusercontent.com/abubakarsiddik31/claude-skills-collection/main/collections.json";

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
        return Err(format!(
            "catalog manifest version {} is not supported",
            raw.version
        ));
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

/// Remote manifest → cached manifest → bundled fallback. A cached
/// manifest younger than the 24h TTL (`CACHE_TTL_SECS`) is served
/// without any HTTP; only a stale or missing cache hits the network.
/// A tampered or broken manifest can at worst advertise unhelpful
/// repos; it names repositories only and cannot bypass any install
/// validation.
pub fn load_catalog(
    http: &dyn GithubHttp,
    cache: &mut CollectionsCache,
    now: u64,
) -> (Vec<ManifestCollection>, CatalogSource) {
    if let Some(cached) = &cache.manifest {
        if super::store::cache_fresh(cached.fetched_at, now) {
            if let Ok(collections) = parse_manifest(&cached.raw) {
                return (collections, CatalogSource::Cached);
            }
        }
    }
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
        assert!(
            cols.len() >= 3,
            "fallback should seed the three verified repos"
        );
        assert!(cols.iter().any(|c| c.repo == "anthropics/skills"));
        assert!(cols.iter().any(|c| c.repo == "obra/superpowers"));
        assert!(cols.iter().any(|c| c.repo == "mattpocock/skills"));
    }

    use crate::collections::github::testutil::FakeGithubHttp;
    use crate::collections::store::{CollectionsCache, UserCollection, CACHE_TTL_SECS};
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
        let (cols, source) = load_catalog(
            &FakeGithubHttp::new(),
            &mut CollectionsCache::default(),
            1_000,
        );
        assert_eq!(source, CatalogSource::Bundled);
        assert!(cols.len() >= 3);
    }

    #[test]
    fn a_fresh_cache_short_circuits_even_when_the_remote_changed() {
        let mut http = FakeGithubHttp::new();
        http.mount("collections.json", MANIFEST_BODY);
        let mut cache = CollectionsCache::default();

        let (cols, source) = load_catalog(&http, &mut cache, 1_000);
        assert_eq!(source, CatalogSource::Manifest);
        assert_eq!(cols[0].id, "anthropics-skills");

        // The remote serves a DIFFERENT body now, but within the TTL
        // the cached copy must win — no HTTP refetch.
        let other = r#"{"version": 1, "collections": [
            {"id": "other", "title": "Other", "repo": "other/repo"}
        ]}"#;
        http.mount("collections.json", other);
        let (cols, source) = load_catalog(&http, &mut cache, 1_100);
        assert_eq!(source, CatalogSource::Cached);
        assert_eq!(cols.len(), 1);
        assert_eq!(cols[0].id, "anthropics-skills");

        // Past the TTL the remote is fetched again and the new body wins.
        let (cols, source) = load_catalog(&http, &mut cache, 1_000 + CACHE_TTL_SECS + 1);
        assert_eq!(source, CatalogSource::Manifest);
        assert_eq!(cols.len(), 1);
        assert_eq!(cols[0].id, "other");
    }

    #[test]
    fn merge_dedupes_user_added_repos_already_in_the_manifest() {
        let manifest = parse_manifest(MANIFEST_BODY).unwrap();
        let user = vec![
            UserCollection {
                id: "anthropics/skills".into(),
                title: "dupe".into(),
                owner: "anthropics".into(),
                repo: "skills".into(),
                subpath: None,
                added_at: 0,
            },
            UserCollection {
                id: "my/own".into(),
                title: "own".into(),
                owner: "my".into(),
                repo: "own".into(),
                subpath: None,
                added_at: 0,
            },
        ];
        let merged: Vec<CollectionInfo> = merge_collections(&manifest, &user);
        assert_eq!(merged.len(), 2);
        assert!(merged.iter().all(|c| c.builtin == (c.repo != "own")));
    }
}
