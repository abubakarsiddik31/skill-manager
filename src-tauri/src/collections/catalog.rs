use super::github::{GithubHttp, TreeResponse};
use super::RemoteSkill;

/// Derive the installable skills from a recursive git tree: every blob
/// named `SKILL.md` marks its parent folder as a skill, at any depth
/// (anthropic-style `skills/<name>/` and nested
/// `skills/engineering/<name>/` layouts both work). Optionally scoped
/// to a subpath for monorepos.
pub fn skills_from_tree(
    owner: &str,
    repo: &str,
    branch: &str,
    tree: &TreeResponse,
    subpath: Option<&str>,
) -> Vec<RemoteSkill> {
    let prefix = subpath
        .map(|p| p.trim_end_matches('/'))
        .filter(|p| !p.is_empty())
        .map(str::to_string);

    let mut skills: Vec<RemoteSkill> = Vec::new();
    for entry in &tree.tree {
        if entry.kind != "blob" {
            continue;
        }
        let folder = match entry.path.strip_suffix("/SKILL.md") {
            Some(f) => f.to_string(),
            None if entry.path == "SKILL.md" => String::new(),
            None => continue,
        };
        if let Some(p) = &prefix {
            let in_scope = folder == *p || folder.starts_with(&format!("{p}/"));
            if !in_scope {
                continue;
            }
        }
        if skills.iter().any(|s| s.path == folder) {
            continue; // one entry per folder, whatever the tree repeats
        }
        let name = if folder.is_empty() {
            repo.to_string()
        } else {
            folder.rsplit('/').next().unwrap_or(repo).to_string()
        };
        skills.push(RemoteSkill {
            name,
            description: None,
            owner: owner.to_string(),
            repo: repo.to_string(),
            path: folder,
            branch: branch.to_string(),
        });
    }
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    skills
}

/// Resolve the repo's default branch, fetch its recursive tree, and
/// derive the skills. Returns `(branch, tree, skills)` — the tree is
/// needed again at install time for blob SHAs.
pub fn enumerate_repo_skills(
    http: &dyn GithubHttp,
    owner: &str,
    repo: &str,
    subpath: Option<&str>,
) -> Result<(String, TreeResponse, Vec<RemoteSkill>), String> {
    let branch = http.fetch_default_branch(owner, repo)?;
    let tree = http.fetch_tree(owner, repo, &branch)?;
    if tree.truncated {
        return Err(format!(
            "repository '{owner}/{repo}' is too large to enumerate"
        ));
    }
    let skills = skills_from_tree(owner, repo, &branch, &tree, subpath);
    Ok((branch, tree, skills))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::github::parse_tree;
    use crate::collections::github::testutil::FakeGithubHttp;

    const REPO_TREE: &str = r#"{
        "sha": "tree-sha",
        "truncated": false,
        "tree": [
            {"path": "README.md", "mode": "100644", "sha": "b0", "type": "blob"},
            {"path": "skills/brainstorming/SKILL.md", "mode": "100644", "sha": "b1", "type": "blob"},
            {"path": "skills/brainstorming/references/x.md", "mode": "100644", "sha": "b2", "type": "blob"},
            {"path": "skills/engineering/code-review/SKILL.md", "mode": "100644", "sha": "b3", "type": "blob"},
            {"path": "skills/engineering/code-review/scripts/run.py", "mode": "100755", "sha": "b4", "type": "blob"},
            {"path": "skills/engineering", "mode": "040000", "sha": "t1", "type": "tree"},
            {"path": "docs/skill.md", "mode": "100644", "sha": "b5", "type": "blob"}
        ]
    }"#;

    fn tree() -> TreeResponse {
        parse_tree(REPO_TREE).unwrap()
    }

    #[test]
    fn finds_skill_folders_at_any_depth() {
        let skills = skills_from_tree("obra", "superpowers", "main", &tree(), None);
        let names: Vec<&str> = skills.iter().map(|s| s.name.as_str()).collect();
        // nested `skills/engineering/code-review` and flat
        // `skills/brainstorming` both count; `docs/skill.md` (wrong file
        // name) and plain files do not
        assert_eq!(names, ["brainstorming", "code-review"]);
    }

    #[test]
    fn skills_carry_full_source_coordinates() {
        let skills = skills_from_tree("obra", "superpowers", "main", &tree(), None);
        let review = skills.iter().find(|s| s.name == "code-review").unwrap();
        assert_eq!(review.owner, "obra");
        assert_eq!(review.repo, "superpowers");
        assert_eq!(review.path, "skills/engineering/code-review");
        assert_eq!(review.branch, "main");
        assert!(review.description.is_none());
    }

    #[test]
    fn subpath_scopes_enumeration() {
        let skills = skills_from_tree(
            "obra",
            "superpowers",
            "main",
            &tree(),
            Some("skills/engineering"),
        );
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "code-review");
        // a trailing slash on the subpath must not break matching
        let skills = skills_from_tree(
            "obra",
            "superpowers",
            "main",
            &tree(),
            Some("skills/engineering/"),
        );
        assert_eq!(skills.len(), 1);
    }

    #[test]
    fn root_skill_md_makes_the_repo_a_single_skill() {
        let single = r#"{"sha": "x", "tree": [
            {"path": "SKILL.md", "mode": "100644", "sha": "b1", "type": "blob"},
            {"path": "scripts/run.py", "mode": "100644", "sha": "b2", "type": "blob"}
        ]}"#;
        let skills = skills_from_tree(
            "me",
            "one-skill-repo",
            "main",
            &parse_tree(single).unwrap(),
            None,
        );
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].path, "");
        assert_eq!(skills[0].name, "one-skill-repo");
    }

    #[test]
    fn enumerate_resolves_branch_then_tree() {
        let mut http = FakeGithubHttp::new();
        http.mount("/repos/obra/superpowers", r#"{"default_branch": "trunk"}"#);
        http.mount("recursive=1", REPO_TREE);
        let (branch, tree, skills) =
            enumerate_repo_skills(&http, "obra", "superpowers", None).unwrap();
        assert_eq!(branch, "trunk");
        assert_eq!(tree.sha, "tree-sha");
        assert_eq!(skills.len(), 2);
    }

    #[test]
    fn enumerate_rejects_truncated_trees() {
        let mut http = FakeGithubHttp::new();
        http.mount("/repos/a/b", r#"{"default_branch": "main"}"#);
        http.mount(
            "recursive=1",
            r#"{"sha": "x", "truncated": true, "tree": []}"#,
        );
        let err = enumerate_repo_skills(&http, "a", "b", None).unwrap_err();
        assert!(err.contains("too large"), "unexpected error: {err}");
    }
}
