use super::{home_dir, AgentTool, SkillAdapter};
use std::path::PathBuf;

/// Claude Code / Claude.ai skills: `~/.claude/skills/<name>/SKILL.md`.
/// This is the fully-supported reference adapter — the other tools follow
/// the same shape but their directory conventions are less standardized.
pub struct ClaudeAdapter;

impl SkillAdapter for ClaudeAdapter {
    fn tool(&self) -> AgentTool {
        AgentTool::Claude
    }

    fn skills_dir(&self) -> PathBuf {
        home_dir().join(".claude").join("skills")
    }

    fn project_subpath(&self) -> &'static str {
        ".claude/skills"
    }
}
