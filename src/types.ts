export type AgentTool =
  | "claude"
  | "agents"
  | "copilot"
  | "crush"
  | "cursor"
  | "gemini"
  | "opencode"
  | "roo";

/** How a tool relates to a skills folder it reads. */
export type FolderRole = "own" | "compat";

export interface ToolFolderInfo {
  tool: AgentTool;
  dir: string;
  role: FolderRole;
  dirExists: boolean;
}

/** A coding agent and every skills folder it reads (own + compat). */
export interface ToolEntry {
  id: string;
  label: string;
  folders: ToolFolderInfo[];
}

export type SkillScope = "user" | "project";

export interface Skill {
  id: string;
  tool: AgentTool;
  name: string;
  description: string;
  path: string;
  scope: SkillScope;
  enabled: boolean;
}

export interface ProjectInfo {
  path: string;
  name: string;
}

export type View = { kind: "global" } | { kind: "project"; project: ProjectInfo };

