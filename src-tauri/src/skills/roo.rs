use super::{home_dir, AgentTool, SkillAdapter};
use std::path::PathBuf;

/// Roo Code agent skills. Per Roo's docs (docs.roocode.com/features/skills),
/// global skills live in `~/.roo/skills` and project skills in
/// `.roo/skills`; the cross-agent `~/.agents/skills` / `.agents/skills`
/// paths are read at a lower priority and covered by the agents adapter.
pub struct RooAdapter;

impl SkillAdapter for RooAdapter {
    fn tool(&self) -> AgentTool {
        AgentTool::Roo
    }

    fn skills_dir(&self) -> PathBuf {
        home_dir().join(".roo").join("skills")
    }

    fn project_subpath(&self) -> &'static str {
        ".roo/skills"
    }
}
