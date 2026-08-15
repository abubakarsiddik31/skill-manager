use super::{home_dir, AgentTool, SkillAdapter};
use std::path::PathBuf;

/// Gemini CLI skills. Per Google's docs (google-gemini/gemini-cli,
/// docs/cli/skills.md), user skills live in `~/.gemini/skills` and
/// workspace skills in `.gemini/skills`, following the Agent Skills
/// open standard (`<name>/SKILL.md`). Each tier also reads a
/// `~/.agents/skills` / `.agents/skills` alias, but those directories
/// are already covered by the Codex adapter, so this adapter
/// deliberately scans only `.gemini` to avoid listing skills twice.
pub struct GeminiAdapter;

impl SkillAdapter for GeminiAdapter {
    fn tool(&self) -> AgentTool {
        AgentTool::Gemini
    }

    fn skills_dir(&self) -> PathBuf {
        home_dir().join(".gemini").join("skills")
    }

    fn project_subpath(&self) -> &'static str {
        ".gemini/skills"
    }
}
