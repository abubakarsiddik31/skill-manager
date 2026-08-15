use serde::Serialize;
use super::{adapter_for, AgentTool};

/// How a tool relates to a skills folder. `Own` is the directory the tool
/// documents as its primary location (and where it installs skills);
/// `Compat` is a shared or alias path the tool also scans.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FolderRole {
    Own,
    Compat,
}

pub struct FolderRef {
    pub folder: AgentTool,
    pub role: FolderRole,
}

const fn own(folder: AgentTool) -> FolderRef {
    FolderRef { folder, role: FolderRole::Own }
}

const fn compat(folder: AgentTool) -> FolderRef {
    FolderRef { folder, role: FolderRole::Compat }
}

/// A coding agent in the registry. Tools are pure readers of folders —
/// several (Goose, Amp) have no folder of their own and only read the
/// shared `~/.agents/skills` directory, so they exist only in this table
/// and get no `SkillAdapter`. Sources for each mapping are cited in the
/// adapter doc comments and the README's supported-tools table.
pub struct ToolDef {
    pub id: &'static str,
    pub label: &'static str,
    pub folders: &'static [FolderRef],
}

pub const TOOLS: &[ToolDef] = &[
    // Claude Code: ~/.claude/skills only.
    ToolDef { id: "claude-code", label: "Claude Code", folders: &[own(AgentTool::Claude)] },
    // Codex: ~/.agents/skills is its only location.
    ToolDef { id: "codex", label: "Codex", folders: &[own(AgentTool::Agents)] },
    ToolDef { id: "cursor", label: "Cursor", folders: &[own(AgentTool::Cursor)] },
    // Gemini CLI: ~/.gemini/skills, with ~/.agents/skills as an alias.
    ToolDef {
        id: "gemini",
        label: "Gemini CLI",
        folders: &[own(AgentTool::Gemini), compat(AgentTool::Agents)],
    },
    // OpenCode: ~/.config/opencode/skills, also scans ~/.claude/skills.
    ToolDef {
        id: "opencode",
        label: "OpenCode",
        folders: &[own(AgentTool::Opencode), compat(AgentTool::Claude)],
    },
    // VS Code / GitHub Copilot: personal skills in ~/.copilot/skills, also
    // reads ~/.agents/skills and ~/.claude/skills.
    ToolDef {
        id: "copilot",
        label: "VS Code / Copilot",
        folders: &[own(AgentTool::Copilot), compat(AgentTool::Agents), compat(AgentTool::Claude)],
    },
    // Goose: recommends ~/.agents/skills (legacy .goose/skills still read).
    ToolDef { id: "goose", label: "Goose", folders: &[own(AgentTool::Agents)] },
    // Amp: top preference is the shared agents folders; its own
    // ~/.config/amp/skills is the lowest-priority fallback, not scanned.
    ToolDef { id: "amp", label: "Amp", folders: &[own(AgentTool::Agents)] },
    // Crush: ~/.config/crush/skills, also scans the shared, Claude, and
    // Cursor folders.
    ToolDef {
        id: "crush",
        label: "Crush",
        folders: &[
            compat(AgentTool::Agents),
            compat(AgentTool::Claude),
            compat(AgentTool::Cursor),
        ],
    },
    // Roo Code: ~/.roo/skills (higher priority), .agents/skills (shared).
    ToolDef { id: "roo", label: "Roo Code", folders: &[compat(AgentTool::Agents)] },
    ToolDef { id: "kiro", label: "Kiro", folders: &[] },
    ToolDef { id: "junie", label: "Junie", folders: &[] },
    // Factory Droid: ~/.factory/skills, .agents/skills as compatibility.
    ToolDef { id: "factory", label: "Factory Droid", folders: &[compat(AgentTool::Agents)] },
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolFolderInfo {
    pub tool: AgentTool,
    pub dir: String,
    pub role: FolderRole,
    pub dir_exists: bool,
}

/// A sidebar entry: a coding agent plus every skills folder it reads,
/// resolved against the adapters so the frontend gets paths and
/// existence in one call.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolEntry {
    pub id: String,
    pub label: String,
    pub folders: Vec<ToolFolderInfo>,
}

pub fn tool_entries() -> Vec<ToolEntry> {
    TOOLS
        .iter()
        .map(|t| ToolEntry {
            id: t.id.to_string(),
            label: t.label.to_string(),
            folders: t
                .folders
                .iter()
                .map(|f| {
                    let adapter = adapter_for(f.folder);
                    let dir = adapter.skills_dir();
                    ToolFolderInfo {
                        tool: f.folder,
                        dir: dir.to_string_lossy().to_string(),
                        role: f.role,
                        dir_exists: dir.is_dir(),
                    }
                })
                .collect(),
        })
        .collect()
}
