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
    let compact: String = raw
        .content
        .chars()
        .filter(|c| *c != '\n' && *c != '\r')
        .collect();
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

    fn fetch_tree(&self, owner: &str, repo: &str, reference: &str) -> Result<TreeResponse, String> {
        let url = api_url(&format!(
            "/repos/{owner}/{repo}/git/trees/{reference}?recursive=1"
        ));
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
