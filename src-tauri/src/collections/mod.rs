pub mod catalog;
pub mod github;

pub use catalog::{enumerate_repo_skills, skills_from_tree};
pub use github::{GithubHttp, TreeEntry, TreeResponse, UreqGithubHttp};

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
