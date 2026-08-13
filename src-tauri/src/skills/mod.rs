mod claude;
mod codex;
mod cursor;
mod opencode;

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub use claude::ClaudeAdapter;
pub use codex::CodexAdapter;
pub use cursor::CursorAdapter;
pub use opencode::OpenCodeAdapter;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AgentTool {
    Claude,
    Codex,
    Cursor,
    Opencode,
}

impl AgentTool {
    pub fn label(&self) -> &'static str {
        match self {
            AgentTool::Claude => "Claude",
            AgentTool::Codex => "Codex",
            AgentTool::Cursor => "Cursor",
            AgentTool::Opencode => "OpenCode",
        }
    }
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolInfo {
    pub tool: AgentTool,
    pub label: String,
    pub skills_dir: String,
    pub dir_exists: bool,
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
    let skills_dir = skill_path
        .parent()
        .and_then(|p| p.parent())
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "invalid skill path"))?;
    let name = skill_path
        .parent()
        .and_then(|p| p.file_name())
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "invalid skill path"))?;

    let (from, to) = if enable {
        (skills_dir.join(DISABLED_DIR).join(name), skills_dir.join(name))
    } else {
        let disabled_root = skills_dir.join(DISABLED_DIR);
        fs::create_dir_all(&disabled_root)?;
        (skills_dir.join(name), disabled_root.join(name))
    };

    fs::rename(&from, &to)?;
    Ok(to.join(MANIFEST_FILE))
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
        Box::new(CodexAdapter),
        Box::new(CursorAdapter),
        Box::new(OpenCodeAdapter),
    ]
}

pub fn adapter_for(tool: AgentTool) -> Box<dyn SkillAdapter> {
    match tool {
        AgentTool::Claude => Box::new(ClaudeAdapter),
        AgentTool::Codex => Box::new(CodexAdapter),
        AgentTool::Cursor => Box::new(CursorAdapter),
        AgentTool::Opencode => Box::new(OpenCodeAdapter),
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
