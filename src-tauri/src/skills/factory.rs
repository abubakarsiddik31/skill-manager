use super::{home_dir, AgentTool, SkillAdapter};
use std::path::PathBuf;

/// Factory Droid agent skills. Per Factory's docs
/// (docs.factory.ai/cli/configuration/skills), personal skills live in
/// `~/.factory/skills` and project skills in `.factory/skills`.
/// Repository-level `.agents/skills` and `~/.agents/skills` are read as
/// compatibility paths and covered by the agents adapter.
pub struct FactoryAdapter;

impl SkillAdapter for FactoryAdapter {
    fn tool(&self) -> AgentTool {
        AgentTool::Factory
    }

    fn skills_dir(&self) -> PathBuf {
        home_dir().join(".factory").join("skills")
    }

    fn project_subpath(&self) -> &'static str {
        ".factory/skills"
    }
}
