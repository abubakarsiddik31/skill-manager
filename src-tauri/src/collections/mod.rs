pub mod catalog;
pub mod github;
pub mod manifest;

pub use catalog::{enumerate_repo_skills, skills_from_tree};
pub use github::{GithubHttp, TreeEntry, TreeResponse, UreqGithubHttp};
pub use manifest::{
    parse_manifest, CatalogSource, ManifestCollection, BUNDLED_MANIFEST, CATALOG_URL,
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
