use super::{home_dir, AgentTool, SkillAdapter};
use std::path::PathBuf;

/// The shared `~/.agents/skills` folder from the Agent Skills open standard
/// (agentskills.io). Codex, Goose, and Amp read it as their primary skills
/// location; Gemini CLI, VS Code, Crush, Roo Code, and Factory scan it as a
/// compatibility path (see `tools.rs` for the full mapping). No single tool
/// owns it, so Skill Manager surfaces it as its own entry.
pub struct AgentsAdapter;

impl SkillAdapter for AgentsAdapter {
    fn tool(&self) -> AgentTool {
        AgentTool::Agents
    }

    fn skills_dir(&self) -> PathBuf {
        home_dir().join(".agents").join("skills")
    }

    fn project_subpath(&self) -> &'static str {
        ".agents/skills"
    }
}
