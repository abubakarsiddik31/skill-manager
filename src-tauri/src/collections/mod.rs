pub mod catalog;
pub mod github;
pub mod install;
pub mod manifest;
pub mod store;

pub use catalog::{enumerate_repo_skills, skills_from_tree};
pub use github::{GithubHttp, TreeEntry, TreeResponse, UreqGithubHttp};
pub use install::{
    files_for_skill, install_skill_files, safe_relative, Provenance, RemoteFile, PROVENANCE_FILE,
};
pub use manifest::{
    load_catalog, merge_collections, parse_manifest, CatalogSource, ManifestCollection,
    BUNDLED_MANIFEST, CATALOG_URL,
};
pub use store::{
    add_user_collection, cache_fresh, load_cache, load_user_collections, now_secs,
    remove_user_collection, save_cache, save_user_collections, CollectionsCache, ManifestCache,
    RepoCache, UserCollection,
};

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
