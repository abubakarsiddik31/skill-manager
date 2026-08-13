use super::{home_dir, AgentTool, SkillAdapter};
use std::path::PathBuf;

/// Codex CLI skills. Codex does not (yet) have a single documented skills
/// directory the way Claude Code does, so this points at the most likely
/// user-level location and follows Claude's `SKILL.md` shape. Update this
/// path if/when Codex publishes an official convention.
pub struct CodexAdapter;

impl SkillAdapter for CodexAdapter {
    fn tool(&self) -> AgentTool {
        AgentTool::Codex
    }

    fn skills_dir(&self) -> PathBuf {
        home_dir().join(".codex").join("skills")
    }

    fn project_subpath(&self) -> &'static str {
        ".codex/skills"
    }
}
