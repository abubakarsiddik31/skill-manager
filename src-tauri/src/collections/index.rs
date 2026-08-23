use std::collections::HashMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use super::RemoteSkill;

/// The skill index compiled into the binary: names, descriptions, and
/// paths for every built-in collection, generated from each repo's git
/// tree. Browsing serves this file and touches no GitHub API — the
/// unauthenticated rate limit (60 req/h) is only spent on installs and
/// explicit refreshes.
pub const BUNDLED_INDEX: &str = include_str!("skills.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexedSkill {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Folder containing the SKILL.md; "" when the repo root is the skill.
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexedRepo {
    pub branch: String,
    pub skills: Vec<IndexedSkill>,
}

/// Parse and validate a bundled index. Only `version: 1` is understood;
/// every key must be a legal `owner/repo` slug so a malformed file can
/// never be coaxed into describing an arbitrary repository.
pub fn parse_skill_index(json: &str) -> Result<HashMap<String, IndexedRepo>, String> {
    #[derive(Deserialize)]
    struct Raw {
        version: u32,
        repos: HashMap<String, IndexedRepo>,
    }
    let raw: Raw =
        serde_json::from_str(json).map_err(|e| format!("bundled skill index is malformed: {e}"))?;
    if raw.version != 1 {
        return Err(format!(
            "bundled skill index version {} is not supported",
            raw.version
        ));
    }
    for key in raw.repos.keys() {
        if super::split_repo(key).is_none() {
            return Err(format!(
                "bundled skill index has an invalid repo key '{key}'"
            ));
        }
    }
    Ok(raw.repos)
}

fn index() -> Option<&'static HashMap<String, IndexedRepo>> {
    static PARSED: OnceLock<Option<HashMap<String, IndexedRepo>>> = OnceLock::new();
    PARSED
        .get_or_init(|| parse_skill_index(BUNDLED_INDEX).ok())
        .as_ref()
}

/// The bundled skills for one repo, scoped by the same subpath rule as
/// `skills_from_tree`. `None` when the repo is not in the index — the
/// caller must enumerate it live instead.
pub fn bundled_skills(owner: &str, repo: &str, subpath: Option<&str>) -> Option<Vec<RemoteSkill>> {
    let indexed = index()?.get(&format!("{owner}/{repo}"))?;
    let prefix = subpath
        .map(|p| p.trim_end_matches('/'))
        .filter(|p| !p.is_empty())
        .map(str::to_string);

    let mut skills: Vec<RemoteSkill> = indexed
        .skills
        .iter()
        .filter(|s| {
            prefix
                .as_ref()
                .is_none_or(|p| s.path == *p || s.path.starts_with(&format!("{p}/")))
        })
        .map(|s| RemoteSkill {
            name: s.name.clone(),
            description: s.description.clone(),
            owner: owner.to_string(),
            repo: repo.to_string(),
            path: s.path.clone(),
            branch: indexed.branch.clone(),
        })
        .collect();
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    Some(skills)
}

/// How many skills the bundled index lists for a repo (after subpath
/// scoping), so `list_collections` can show counts offline.
pub fn bundled_skill_count(owner: &str, repo: &str, subpath: Option<&str>) -> Option<u64> {
    bundled_skills(owner, repo, subpath).map(|s| s.len() as u64)
}

/// Fill missing descriptions on freshly enumerated skills from the
/// bundled index, matched by path — a refresh keeps the UI readable
/// without spending a blob fetch per skill.
pub fn enrich_descriptions(owner: &str, repo: &str, skills: &mut [RemoteSkill]) {
    let Some(indexed) = index().and_then(|i| i.get(&format!("{owner}/{repo}"))) else {
        return;
    };
    let by_path: HashMap<&str, &IndexedSkill> = indexed
        .skills
        .iter()
        .map(|s| (s.path.as_str(), s))
        .collect();
    for skill in skills.iter_mut() {
        if skill.description.is_none() {
            if let Some(hit) = by_path.get(skill.path.as_str()) {
                skill.description = hit.description.clone();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_index_parses_and_covers_the_seed_repos() {
        let repos = parse_skill_index(BUNDLED_INDEX).unwrap();
        assert!(repos.contains_key("anthropics/skills"));
        assert!(repos.contains_key("obra/superpowers"));
        assert!(repos.contains_key("mattpocock/skills"));
        for (slug, repo) in &repos {
            assert!(!repo.branch.is_empty(), "{slug} needs a branch");
            assert!(!repo.skills.is_empty(), "{slug} has no skills");
            for skill in &repo.skills {
                assert!(!skill.name.is_empty(), "{slug} has an unnamed skill");
            }
        }
    }

    #[test]
    fn bundled_skills_carry_descriptions_and_sort_by_name() {
        let skills = bundled_skills("obra", "superpowers", None).unwrap();
        assert!(skills.len() > 10);
        let sorted = skills.iter().map(|s| s.name.clone()).collect::<Vec<_>>();
        let mut expected = sorted.clone();
        expected.sort();
        assert_eq!(sorted, expected);
        assert!(skills.iter().all(|s| s.description.is_some()));
    }

    #[test]
    fn unknown_repos_have_no_bundled_skills() {
        assert!(bundled_skills("someone", "unknown", None).is_none());
        assert!(bundled_skill_count("someone", "unknown", None).is_none());
    }

    #[test]
    fn subpath_scopes_the_bundled_listing() {
        let all = bundled_skills("anthropics", "skills", None).unwrap();
        let scoped = bundled_skills("anthropics", "skills", Some("skills")).unwrap();
        // the repo also ships a template/ skill that falls outside skills/
        assert!(scoped.len() < all.len());
        assert!(scoped.iter().all(|s| s.path.starts_with("skills/")));
        let scoped =
            bundled_skills("anthropics", "skills", Some("skills/random")).unwrap_or_default();
        assert!(scoped.is_empty());
        // trailing slash must behave the same as no slash
        assert_eq!(
            bundled_skills("anthropics", "skills", Some("skills/"))
                .unwrap()
                .len(),
            bundled_skills("anthropics", "skills", Some("skills"))
                .unwrap()
                .len()
        );
    }

    #[test]
    fn rejects_unsupported_versions_and_bad_keys() {
        assert!(parse_skill_index(r#"{"version": 2, "repos": {}}"#).is_err());
        assert!(parse_skill_index(
            r#"{"version": 1, "repos": {"not-a-slug": {"branch": "main", "skills": []}}}"#
        )
        .is_err());
    }

    #[test]
    fn enrich_fills_only_matching_paths() {
        let mut skills = vec![
            RemoteSkill {
                name: "known".into(),
                description: None,
                owner: "obra".into(),
                repo: "superpowers".into(),
                path: "skills/brainstorming".into(),
                branch: "main".into(),
            },
            RemoteSkill {
                name: "fresh-upstream".into(),
                description: None,
                owner: "obra".into(),
                repo: "superpowers".into(),
                path: "skills/added-after-release".into(),
                branch: "main".into(),
            },
        ];
        enrich_descriptions("obra", "superpowers", &mut skills);
        assert!(skills[0].description.is_some());
        assert!(skills[1].description.is_none());
        // a repo absent from the index is a no-op, not an error
        enrich_descriptions("someone", "unknown", &mut skills);
        assert!(skills[0].description.is_some());
    }
}
