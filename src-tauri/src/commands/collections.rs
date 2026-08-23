use super::{find_skill_by_manifest, resolve_target_root, skills_roots};
use crate::collections::github::{GithubHttp, UreqGithubHttp};
use crate::collections::{
    self, CatalogSource, CollectionInfo, CollectionsCache, ManifestCollection, Provenance,
    RemoteSkill,
};
use crate::projects;
use crate::skills::{self, Skill};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

const HTTP: UreqGithubHttp = UreqGithubHttp;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListCollectionsResult {
    pub collections: Vec<CollectionInfo>,
    pub source: CatalogSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallResult {
    pub skill: Skill,
    pub skipped_links: u64,
}

fn config_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn catalog_and_user(
    app: &AppHandle,
    cache: &mut CollectionsCache,
) -> Result<(Vec<ManifestCollection>, Vec<collections::UserCollection>), String> {
    let dir = config_dir(app)?;
    let (manifest, _) = collections::load_catalog(&HTTP, cache, collections::now_secs());
    Ok((manifest, collections::load_user_collections(&dir)))
}

fn find_collection(
    app: &AppHandle,
    cache: &mut CollectionsCache,
    id: &str,
) -> Result<CollectionInfo, String> {
    let (manifest, user) = catalog_and_user(app, cache)?;
    collections::merge_collections(&manifest, &user)
        .into_iter()
        .find(|c| c.id == id)
        .ok_or_else(|| format!("no collection named '{id}'"))
}

/// One repo's `(branch, tree, from_cache)` with cache-first semantics;
/// `force` bypasses freshness for the refresh button. When the network
/// fails but a stale cache entry exists, the stale tree is served —
/// browsing degrades instead of erroring (the spec's fallback chain).
fn repo_tree(
    app: &AppHandle,
    cache: &mut CollectionsCache,
    owner: &str,
    repo: &str,
    force: bool,
) -> Result<(String, collections::TreeResponse, bool), String> {
    let key = format!("{owner}/{repo}");
    if !force {
        if let Some(hit) = cache.repos.get(&key) {
            if collections::cache_fresh(hit.fetched_at, collections::now_secs()) {
                return Ok((hit.branch.clone(), hit.tree.clone(), true));
            }
        }
    }
    match collections::enumerate_repo_skills(&HTTP, owner, repo, None) {
        Ok((branch, tree, _)) => {
            cache.repos.insert(
                key,
                collections::RepoCache {
                    fetched_at: collections::now_secs(),
                    branch: branch.clone(),
                    tree: tree.clone(),
                },
            );
            let _ = collections::save_cache(&config_dir(app)?, cache);
            Ok((branch, tree, false))
        }
        Err(e) => match cache.repos.get(&key) {
            Some(hit) => Ok((hit.branch.clone(), hit.tree.clone(), true)),
            None => Err(e),
        },
    }
}

#[tauri::command]
pub fn list_collections(app: AppHandle) -> Result<ListCollectionsResult, String> {
    let dir = config_dir(&app)?;
    let mut cache = collections::load_cache(&dir);
    let (manifest, source) = collections::load_catalog(&HTTP, &mut cache, collections::now_secs());
    let _ = collections::save_cache(&dir, &cache);
    let user = collections::load_user_collections(&dir);
    let mut merged = collections::merge_collections(&manifest, &user);
    for collection in &mut merged {
        // Counts come offline first — the bundled index covers the
        // built-ins; only user-added repos lean on the tree cache.
        if let Some(count) = collections::bundled_skill_count(
            &collection.owner,
            &collection.repo,
            collection.subpath.as_deref(),
        ) {
            collection.skill_count = Some(count);
            continue;
        }
        let key = format!("{}/{}", collection.owner, collection.repo);
        if let Some(hit) = cache.repos.get(&key) {
            let skills = collections::skills_from_tree(
                &collection.owner,
                &collection.repo,
                &hit.branch,
                &hit.tree,
                collection.subpath.as_deref(),
            );
            collection.skill_count = Some(skills.len() as u64);
        }
    }
    Ok(ListCollectionsResult {
        collections: merged,
        source,
    })
}

fn browse(app: AppHandle, id: String, force: bool) -> Result<Vec<RemoteSkill>, String> {
    let dir = config_dir(&app)?;
    let mut cache = collections::load_cache(&dir);
    let info = find_collection(&app, &mut cache, &id)?;
    // Collections covered by the bundled index browse completely
    // offline — the 60 req/h unauthenticated GitHub limit is reserved
    // for installs and explicit refreshes. `force` (the refresh
    // button) deliberately bypasses this for a live re-enumeration.
    if !force {
        if let Some(skills) =
            collections::bundled_skills(&info.owner, &info.repo, info.subpath.as_deref())
        {
            return Ok(skills);
        }
    }
    let (branch, tree, _) = repo_tree(&app, &mut cache, &info.owner, &info.repo, force)?;
    let mut skills = collections::skills_from_tree(
        &info.owner,
        &info.repo,
        &branch,
        &tree,
        info.subpath.as_deref(),
    );
    // A live enumeration carries no descriptions; graft the bundled
    // ones on by path so refreshes don't blank out the grid.
    collections::enrich_descriptions(&info.owner, &info.repo, &mut skills);
    Ok(skills)
}

#[tauri::command]
pub fn browse_collection(app: AppHandle, id: String) -> Result<Vec<RemoteSkill>, String> {
    browse(app, id, false)
}

#[tauri::command]
pub fn refresh_collection(app: AppHandle, id: String) -> Result<Vec<RemoteSkill>, String> {
    browse(app, id, true)
}

#[tauri::command]
pub fn add_collection(
    app: AppHandle,
    repo: String,
    title: Option<String>,
) -> Result<CollectionInfo, String> {
    let dir = config_dir(&app)?;
    let trimmed = repo.trim().trim_end_matches('/');
    let Some((owner, name)) = collections::split_repo(trimmed) else {
        return Err(format!(
            "'{repo}' is not a valid owner/repo repository slug"
        ));
    };
    // Refuse repos the catalog already offers: merge would dedupe the
    // entry out and the stored row would linger invisibly forever.
    let mut scratch = collections::load_cache(&dir);
    let (manifest, user) = catalog_and_user(&app, &mut scratch)?;
    if collections::merge_collections(&manifest, &user)
        .iter()
        .any(|c| c.owner == owner && c.repo == name)
    {
        return Err(format!(
            "'{owner}/{name}' is already available as a built-in collection"
        ));
    }
    // Probe before persisting: a typo'd or unreachable repo must not
    // leave a phantom entry behind when the command errors.
    HTTP.fetch_default_branch(&owner, &name)?;
    let entry = collections::add_user_collection(&dir, &repo, title)?;
    Ok(CollectionInfo {
        id: entry.id,
        title: entry.title,
        owner: entry.owner,
        repo: entry.repo,
        subpath: entry.subpath,
        builtin: false,
        skill_count: None,
    })
}

#[tauri::command]
pub fn remove_collection(app: AppHandle, id: String) -> Result<(), String> {
    let dir = config_dir(&app)?;
    let mut cache = collections::load_cache(&dir);
    let info = find_collection(&app, &mut cache, &id)?;
    if info.builtin {
        return Err("built-in collections cannot be removed".into());
    }
    collections::remove_user_collection(&dir, &id)
}

#[tauri::command]
pub fn install_skill(
    app: AppHandle,
    tool: skills::AgentTool,
    scope: skills::SkillScope,
    project_path: Option<String>,
    skill: RemoteSkill,
    collection_id: String,
    overwrite: Option<bool>,
) -> Result<InstallResult, String> {
    let tracked = projects::list(&app).unwrap_or_default();
    let root = resolve_target_root(&tracked, tool, scope, project_path)?;
    let roots = skills_roots(&tracked);

    let dir = config_dir(&app)?;
    let mut cache = collections::load_cache(&dir);
    // Fail fast on unknown ids or a skill/collection mismatch, before
    // any network fetch.
    let info = find_collection(&app, &mut cache, &collection_id)?;
    if info.owner != skill.owner || info.repo != skill.repo {
        return Err(format!(
            "skill '{}/{}' does not belong to collection '{collection_id}'",
            skill.owner, skill.repo
        ));
    }
    let (branch, tree, _) = repo_tree(&app, &mut cache, &skill.owner, &skill.repo, false)?;

    let (files, skipped_links) = collections::files_for_skill(&tree, &skill.path)?;
    let provenance = Provenance {
        owner: skill.owner.clone(),
        repo: skill.repo.clone(),
        path: skill.path.clone(),
        branch,
        tree_sha: tree.sha.clone(),
        installed_at: collections::now_secs(),
        collection_id,
    };
    let manifest = collections::install_skill_files(
        &HTTP,
        &roots,
        &root,
        &skill.name,
        &files,
        &provenance,
        overwrite.unwrap_or(false),
    )?;

    if let Some(project) = tracked.iter().find(|p| manifest.starts_with(&p.path)) {
        let _ = projects::clear_skill_count(&app, &project.path);
    }
    let installed = find_skill_by_manifest(&app, &manifest)
        .ok_or_else(|| "skill not found after installation".to_string())?;
    Ok(InstallResult {
        skill: installed,
        skipped_links: skipped_links as u64,
    })
}
