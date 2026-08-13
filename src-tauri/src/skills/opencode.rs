use super::{home_dir, AgentTool, SkillAdapter};
use std::path::PathBuf;

/// OpenCode skills. Points at the most likely user-level location and
/// follows Claude's `SKILL.md` shape. Update this path once OpenCode's
/// skills format is confirmed.
pub struct OpenCodeAdapter;

impl SkillAdapter for OpenCodeAdapter {
    fn tool(&self) -> AgentTool {
        AgentTool::Opencode
    }

    fn skills_dir(&self) -> PathBuf {
        home_dir().join(".config").join("opencode").join("skills")
    }

    fn project_subpath(&self) -> &'static str {
        ".opencode/skills"
    }
}
