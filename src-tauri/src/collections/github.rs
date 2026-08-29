use serde::{Deserialize, Serialize};
use std::io::Read;
use std::time::Duration;

/// GitHub requires a User-Agent on every request; identify the app.
pub const USER_AGENT: &str = "skill-manager (https://github.com/abubakarsiddik31/skill-manager)";

/// Hard cap on any single response. Repo tarballs run a few MB; a
/// misbehaving or hostile server must not be able to stream unbounded
/// data into memory.
const MAX_DOWNLOAD_BYTES: u64 = 64 * 1024 * 1024;

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

/// codeload serves repo archives outside the REST API — and therefore
/// outside its 60 req/h unauthenticated quota. `reference` may be a
/// branch, tag, commit sha, or `HEAD` for the current default branch.
pub fn tarball_url(owner: &str, repo: &str, reference: &str) -> String {
    format!("https://codeload.github.com/{owner}/{repo}/tar.gz/{reference}")
}

pub fn parse_tree(json: &str) -> Result<TreeResponse, String> {
    serde_json::from_str(json).map_err(|e| format!("cannot parse tree response: {e}"))
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
    fn get_bytes(&self, url: &str) -> Result<Vec<u8>, String>;

    fn get_text(&self, url: &str) -> Result<String, String> {
        self.get_bytes(url)
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
    }

    fn fetch_tree(&self, owner: &str, repo: &str, reference: &str) -> Result<TreeResponse, String> {
        let url = api_url(&format!(
            "/repos/{owner}/{repo}/git/trees/{reference}?recursive=1"
        ));
        self.get_text(&url).and_then(|t| parse_tree(&t))
    }

    fn fetch_default_branch(&self, owner: &str, repo: &str) -> Result<String, String> {
        let url = api_url(&format!("/repos/{owner}/{repo}"));
        self.get_text(&url).and_then(|t| parse_default_branch(&t))
    }

    /// Download one repo's tarball — a single request that spends none
    /// of the REST API quota, unlike fetching every skill file as its
    /// own blob.
    fn fetch_tarball(&self, owner: &str, repo: &str, reference: &str) -> Result<Vec<u8>, String> {
        self.get_bytes(&tarball_url(owner, repo, reference))
    }
}

pub struct UreqGithubHttp;

impl UreqGithubHttp {
    fn read_capped(response: ureq::Response) -> Result<Vec<u8>, String> {
        let mut bytes = Vec::new();
        let mut reader = response.into_reader().take(MAX_DOWNLOAD_BYTES + 1);
        reader
            .read_to_end(&mut bytes)
            .map_err(|e| format!("cannot read response: {e}"))?;
        if bytes.len() as u64 > MAX_DOWNLOAD_BYTES {
            return Err(format!(
                "response exceeds the {} MiB download cap",
                MAX_DOWNLOAD_BYTES / (1024 * 1024)
            ));
        }
        Ok(bytes)
    }
}

impl GithubHttp for UreqGithubHttp {
    fn get_bytes(&self, url: &str) -> Result<Vec<u8>, String> {
        match ureq::get(url)
            .set("User-Agent", USER_AGENT)
            .timeout(Duration::from_secs(60))
            .call()
        {
            Ok(response) => Self::read_capped(response),
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
        pub bodies: HashMap<String, Vec<u8>>,
    }

    impl FakeGithubHttp {
        pub fn new() -> Self {
            Self {
                bodies: HashMap::new(),
            }
        }

        pub fn mount(&mut self, url_suffix: &str, body: &str) {
            self.bodies
                .insert(url_suffix.to_string(), body.as_bytes().to_vec());
        }
    }

    impl Default for FakeGithubHttp {
        fn default() -> Self {
            Self::new()
        }
    }

    impl GithubHttp for FakeGithubHttp {
        fn get_bytes(&self, url: &str) -> Result<Vec<u8>, String> {
            self.bodies
                .iter()
                .find(|(suffix, _)| url.ends_with(suffix.as_str()))
                .map(|(_, body)| body.clone())
                .ok_or_else(|| format!("no fixture for {url}"))
        }
    }
}

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
    fn parse_default_branch_reads_field() {
        let json = r#"{"default_branch": "main", "full_name": "a/b"}"#;
        assert_eq!(parse_default_branch(json).unwrap(), "main");
    }

    #[test]
    fn tarball_url_targets_codeload_not_the_api() {
        assert_eq!(
            tarball_url("anthropics", "skills", "HEAD"),
            "https://codeload.github.com/anthropics/skills/tar.gz/HEAD"
        );
    }
}
