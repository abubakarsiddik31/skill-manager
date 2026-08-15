use super::{home_dir, AgentTool, SkillAdapter};
use std::path::PathBuf;

/// VS Code / GitHub Copilot agent skills. Per VS Code's docs
/// (code.visualstudio.com/docs/copilot/customization/agent-skills),
/// personal skills live in `~/.copilot/skills` and project skills in
/// `.github/skills`. VS Code also scans `~/.agents/skills`,
/// `~/.claude/skills`, and their project-level equivalents as
/// compatibility paths — those are covered by the agents and claude
/// adapters, so this adapter only owns the copilot-specific paths.
pub struct CopilotAdapter;

impl SkillAdapter for CopilotAdapter {
    fn tool(&self) -> AgentTool {
        AgentTool::Copilot
    }

    fn skills_dir(&self) -> PathBuf {
        home_dir().join(".copilot").join("skills")
    }

    fn project_subpath(&self) -> &'static str {
        ".github/skills"
    }
}
