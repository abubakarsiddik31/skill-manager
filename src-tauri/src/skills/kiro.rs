use super::{home_dir, AgentTool, SkillAdapter};
use std::path::PathBuf;

/// Kiro (AWS) agent skills. Per Kiro's docs (kiro.dev/docs/skills),
/// global skills live in `~/.kiro/skills` and project skills in
/// `.kiro/skills`.
pub struct KiroAdapter;

impl SkillAdapter for KiroAdapter {
    fn tool(&self) -> AgentTool {
        AgentTool::Kiro
    }

    fn skills_dir(&self) -> PathBuf {
        home_dir().join(".kiro").join("skills")
    }

    fn project_subpath(&self) -> &'static str {
        ".kiro/skills"
    }
}
