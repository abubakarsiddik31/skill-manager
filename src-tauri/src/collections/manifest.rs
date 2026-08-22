use serde::{Deserialize, Serialize};

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
}
