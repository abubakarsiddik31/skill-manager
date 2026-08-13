use super::{home_dir, AgentTool, SkillAdapter};
use std::path::PathBuf;

/// Codex CLI skills. Codex has no `.codex`-specific skills directory -
/// per OpenAI's docs (developers.openai.com/codex/skills), it reads
/// personal skills from `~/.agents/skills` and project skills by walking
/// up from the working directory to `<repo-root>/.agents/skills`.
pub struct CodexAdapter;

impl SkillAdapter for CodexAdapter {
    fn tool(&self) -> AgentTool {
        AgentTool::Codex
    }

    fn skills_dir(&self) -> PathBuf {
        home_dir().join(".agents").join("skills")
    }

    fn project_subpath(&self) -> &'static str {
        ".agents/skills"
    }
}
