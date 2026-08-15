use super::{home_dir, AgentTool, SkillAdapter};
use std::path::PathBuf;

/// Junie (JetBrains) agent skills. Per Junie's docs
/// (junie.jetbrains.com/docs/agent-skills.html), personal skills live in
/// `~/.junie/skills` (`%USERPROFILE%\.junie\skills` on Windows) and
/// project skills in `.junie/skills` under the project root.
pub struct JunieAdapter;

impl SkillAdapter for JunieAdapter {
    fn tool(&self) -> AgentTool {
        AgentTool::Junie
    }

    fn skills_dir(&self) -> PathBuf {
        home_dir().join(".junie").join("skills")
    }

    fn project_subpath(&self) -> &'static str {
        ".junie/skills"
    }
}
