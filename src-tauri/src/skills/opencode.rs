use super::{home_dir, AgentTool, SkillAdapter};
use std::path::PathBuf;

/// OpenCode skills. Verified against OpenCode's docs (opencode.ai/docs/skills):
/// project skills in `.opencode/skills`, global skills in
/// `~/.config/opencode/skills`. OpenCode also reads `.claude/skills` and
/// the shared `.agents/skills` convention, but those are already covered
/// by the Claude and Codex adapters respectively.
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
