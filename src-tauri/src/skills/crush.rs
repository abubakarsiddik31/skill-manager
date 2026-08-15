use super::{home_dir, AgentTool, SkillAdapter};
use std::path::PathBuf;

/// Crush (charmbracelet) agent skills. Per Crush's README
/// (github.com/charmbracelet/crush), user skills live in
/// `$XDG_CONFIG_HOME/crush/skills` (i.e. `~/.config/crush/skills`) and
/// project skills in `.crush/skills`. Crush also scans `~/.agents/skills`,
/// `~/.claude/skills`, and `.cursor/skills` — covered by their own
/// adapters — and `%LOCALAPPDATA%\crush\skills` on Windows.
pub struct CrushAdapter;

impl SkillAdapter for CrushAdapter {
    fn tool(&self) -> AgentTool {
        AgentTool::Crush
    }

    fn skills_dir(&self) -> PathBuf {
        home_dir().join(".config").join("crush").join("skills")
    }

    fn project_subpath(&self) -> &'static str {
        ".crush/skills"
    }
}
