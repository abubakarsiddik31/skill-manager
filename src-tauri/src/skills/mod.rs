mod agents;
mod claude;
mod copilot;
mod crush;
mod cursor;
mod factory;
mod gemini;
mod junie;
mod kiro;
mod opencode;
mod roo;
pub mod tools;

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub use agents::AgentsAdapter;
pub use claude::ClaudeAdapter;
pub use copilot::CopilotAdapter;
pub use crush::CrushAdapter;
pub use cursor::CursorAdapter;
pub use factory::FactoryAdapter;
pub use gemini::GeminiAdapter;
pub use junie::JunieAdapter;
pub use kiro::KiroAdapter;
pub use opencode::OpenCodeAdapter;
pub use roo::RooAdapter;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AgentTool {
    Claude,
    /// The shared `~/.agents/skills` folder — the Agent Skills standard's
    /// interop directory, read natively by Codex, Goose, and Amp. Named
    /// "Agents" rather than after any one tool because it has no owner.
    Agents,
    Copilot,
    Crush,
    Cursor,
    Factory,
    Gemini,
    Junie,
    Kiro,
    Opencode,
    Roo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SkillScope {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "project")]
    Project,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Skill {
    /// Absolute path to the skill's manifest file (SKILL.md). Doubles as
    /// a stable, unique identifier across the app's lifetime.
    pub id: String,
    pub tool: AgentTool,
    pub name: String,
    pub description: String,
    pub path: String,
    pub scope: SkillScope,
    pub enabled: bool,
}

/// Every supported coding agent implements this the same way: a directory
/// of `<skill-name>/SKILL.md` folders. None of these tools ship a native
/// enable/disable switch, so Skill Manager introduces its own convention:
/// disabled skills are moved into a sibling `.disabled/` folder inside the
/// same skills directory. That keeps the operation reversible and leaves
/// the tool's own files untouched otherwise.
pub trait SkillAdapter: Send + Sync {
    fn tool(&self) -> AgentTool;

    /// User-level skills directory for this tool, e.g. `~/.claude/skills`.
    fn skills_dir(&self) -> PathBuf;

    /// Path to this tool's skills directory relative to a project root,
    /// e.g. `.claude/skills`. Used to find project-level skills alongside
    /// the user-level ones from `skills_dir()`.
    fn project_subpath(&self) -> &'static str;

    fn discover(&self) -> Vec<Skill> {
        scan_scope(self.tool(), &self.skills_dir(), SkillScope::User)
    }

    fn discover_at(&self, project_root: &Path) -> Vec<Skill> {
        scan_scope(
            self.tool(),
            &project_root.join(self.project_subpath()),
            SkillScope::Project,
        )
    }
}

fn scan_scope(tool: AgentTool, dir: &Path, scope: SkillScope) -> Vec<Skill> {
    let mut skills = scan_dir(tool, dir, scope.clone(), true);
    skills.extend(scan_dir(tool, &dir.join(DISABLED_DIR), scope, false));
    skills
}

const MANIFEST_FILE: &str = "SKILL.md";
const DISABLED_DIR: &str = ".disabled";

fn scan_dir(tool: AgentTool, dir: &Path, scope: SkillScope, enabled: bool) -> Vec<Skill> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if path.file_name().and_then(|n| n.to_str()) == Some(DISABLED_DIR) {
            continue;
        }
        let manifest = path.join(MANIFEST_FILE);
        if !manifest.is_file() {
            continue;
        }
        let raw = fs::read_to_string(&manifest).unwrap_or_default();
        let (name, description) = parse_frontmatter(&raw);
        let fallback_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        out.push(Skill {
            id: manifest.to_string_lossy().to_string(),
            tool,
            name: if name.is_empty() { fallback_name } else { name },
            description,
            path: path.to_string_lossy().to_string(),
            scope: scope.clone(),
            enabled,
        });
    }
    out
}

/// Minimal YAML frontmatter reader for the two fields Skill Manager cares
/// about (`name`, `description`). Intentionally not a full YAML parser —
/// SKILL.md frontmatter is a flat key: value list.
fn parse_frontmatter(raw: &str) -> (String, String) {
    let mut name = String::new();
    let mut description = String::new();

    let mut lines = raw.lines();
    if lines.next() != Some("---") {
        return (name, description);
    }
    for line in lines {
        if line.trim() == "---" {
            break;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim().trim_matches('"').trim_matches('\'').to_string();
        match key.trim() {
            "name" => name = value,
            "description" => description = value,
            _ => {}
        }
    }
    (name, description)
}

pub fn toggle_enabled(skill_path: &Path, enable: bool) -> std::io::Result<PathBuf> {
    let invalid = || std::io::Error::new(std::io::ErrorKind::NotFound, "invalid skill path");

    let skill_dir = skill_path.parent().ok_or_else(invalid)?;
    let name = skill_dir.file_name().ok_or_else(invalid)?;
    let parent = skill_dir.parent().ok_or_else(invalid)?;

    // A disabled skill's manifest lives one level deeper, under a
    // `.disabled` folder inside the real skills directory - skip that
    // extra segment so `skills_dir` is correct in both directions.
    let skills_dir = if parent.file_name().and_then(|n| n.to_str()) == Some(DISABLED_DIR) {
        parent.parent().ok_or_else(invalid)?
    } else {
        parent
    };

    let (from, to) = if enable {
        (skills_dir.join(DISABLED_DIR).join(name), skills_dir.join(name))
    } else {
        let disabled_root = skills_dir.join(DISABLED_DIR);
        fs::create_dir_all(&disabled_root)?;
        (skills_dir.join(name), disabled_root.join(name))
    };

    move_skill_entry(&from, &to)?;
    Ok(to.join(MANIFEST_FILE))
}

/// Moves a skill folder from `from` to `to`. Plain directories are renamed
/// as usual, but a symlinked skill (common for tools like Cursor that share
/// skills with other tools via a link) needs special care: naively renaming
/// the link node would leave a relative target - e.g. `../../.claude/skills/x`
/// - resolving from the wrong depth once it's nested one level into/out of
/// `.disabled`, silently turning it into a dangling link. Resolve the link's
/// real target first and recreate an absolute symlink at the destination
/// instead.
fn move_skill_entry(from: &Path, to: &Path) -> std::io::Result<()> {
    if from.is_symlink() {
        let target = fs::canonicalize(from)?;
        create_symlink(&target, to)?;
        fs::remove_file(from)
    } else {
        fs::rename(from, to)
    }
}

#[cfg(unix)]
fn create_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
fn create_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    if target.is_dir() {
        std::os::windows::fs::symlink_dir(target, link)
    } else {
        std::os::windows::fs::symlink_file(target, link)
    }
}

pub fn delete_skill_dir(skill_path: &Path) -> std::io::Result<()> {
    let dir = skill_path
        .parent()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "invalid skill path"))?;
    fs::remove_dir_all(dir)
}

pub fn all_adapters() -> Vec<Box<dyn SkillAdapter>> {
    vec![
        Box::new(ClaudeAdapter),
        Box::new(AgentsAdapter),
        Box::new(CopilotAdapter),
        Box::new(CrushAdapter),
        Box::new(CursorAdapter),
        Box::new(FactoryAdapter),
        Box::new(GeminiAdapter),
        Box::new(JunieAdapter),
        Box::new(KiroAdapter),
        Box::new(OpenCodeAdapter),
        Box::new(RooAdapter),
    ]
}

pub fn adapter_for(tool: AgentTool) -> Box<dyn SkillAdapter> {
    match tool {
        AgentTool::Claude => Box::new(ClaudeAdapter),
        AgentTool::Agents => Box::new(AgentsAdapter),
        AgentTool::Copilot => Box::new(CopilotAdapter),
        AgentTool::Crush => Box::new(CrushAdapter),
        AgentTool::Cursor => Box::new(CursorAdapter),
        AgentTool::Factory => Box::new(FactoryAdapter),
        AgentTool::Gemini => Box::new(GeminiAdapter),
        AgentTool::Junie => Box::new(JunieAdapter),
        AgentTool::Kiro => Box::new(KiroAdapter),
        AgentTool::Opencode => Box::new(OpenCodeAdapter),
        AgentTool::Roo => Box::new(RooAdapter),
    }
}

pub fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

pub fn discover_project_skills(project_root: &Path) -> Vec<Skill> {
    all_adapters()
        .into_iter()
        .flat_map(|adapter| adapter.discover_at(project_root))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_skills_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("skill-manager-test-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("demo")).unwrap();
        fs::write(dir.join("demo").join(MANIFEST_FILE), "---\nname: demo\n---\n").unwrap();
        dir
    }

    #[test]
    fn disable_then_enable_round_trips() {
        let skills_dir = temp_skills_dir("roundtrip");
        let manifest = skills_dir.join("demo").join(MANIFEST_FILE);

        let disabled_manifest = toggle_enabled(&manifest, false).expect("disable should succeed");
        assert!(disabled_manifest.starts_with(skills_dir.join(DISABLED_DIR)));
        assert!(disabled_manifest.is_file());

        let re_enabled_manifest =
            toggle_enabled(&disabled_manifest, true).expect("re-enable should succeed");
        assert_eq!(re_enabled_manifest, skills_dir.join("demo").join(MANIFEST_FILE));
        assert!(re_enabled_manifest.is_file());

        fs::remove_dir_all(&skills_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn disabling_a_relative_symlinked_skill_keeps_it_resolvable() {
        // Mirrors a real setup: a skill directory that lives elsewhere,
        // linked into a tool's skills dir with a relative target (e.g.
        // Cursor sharing a skill from `~/.claude/skills` via
        // `../../.claude/skills/<name>`).
        let real_dir = std::env::temp_dir()
            .join(format!("skill-manager-test-real-{}", std::process::id()));
        let _ = fs::remove_dir_all(&real_dir);
        fs::create_dir_all(&real_dir).unwrap();
        fs::write(real_dir.join(MANIFEST_FILE), "---\nname: linked\n---\n").unwrap();

        let skills_dir = temp_skills_dir("symlink");
        let link = skills_dir.join("linked");
        std::os::unix::fs::symlink(&real_dir, &link).unwrap();
        let manifest = link.join(MANIFEST_FILE);

        let disabled_manifest = toggle_enabled(&manifest, false).expect("disable should succeed");
        assert!(disabled_manifest.is_file(), "symlink must still resolve once disabled");

        let re_enabled_manifest =
            toggle_enabled(&disabled_manifest, true).expect("re-enable should succeed");
        assert!(re_enabled_manifest.is_file(), "symlink must still resolve once re-enabled");

        fs::remove_dir_all(&skills_dir).unwrap();
        fs::remove_dir_all(&real_dir).unwrap();
    }
}
