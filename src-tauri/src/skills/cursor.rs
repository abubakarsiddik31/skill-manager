use super::{home_dir, AgentTool, SkillAdapter};
use std::path::PathBuf;

/// Cursor skills. Verified against Cursor's docs (cursor.com/docs/skills):
/// project skills in `.cursor/skills`, personal skills in
/// `~/.cursor/skills`. Cursor also reads the shared `.agents/skills`
/// convention and Claude/Codex's directories for compatibility, but those
/// are already covered by their own adapters, so listing them again here
/// would just duplicate the same skill under two tools.
pub struct CursorAdapter;

impl SkillAdapter for CursorAdapter {
    fn tool(&self) -> AgentTool {
        AgentTool::Cursor
    }

    fn skills_dir(&self) -> PathBuf {
        home_dir().join(".cursor").join("skills")
    }

    fn project_subpath(&self) -> &'static str {
        ".cursor/skills"
    }
}
