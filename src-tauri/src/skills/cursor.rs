use super::{home_dir, AgentTool, SkillAdapter};
use std::path::PathBuf;

/// Cursor skills. Cursor's own convention (rules vs. skills) is still
/// evolving; this points at the most likely user-level location and
/// follows Claude's `SKILL.md` shape. Update this path once Cursor's
/// skills format is confirmed.
pub struct CursorAdapter;

impl SkillAdapter for CursorAdapter {
    fn tool(&self) -> AgentTool {
        AgentTool::Cursor
    }

    fn skills_dir(&self) -> PathBuf {
        home_dir().join(".cursor").join("skills")
    }
}
