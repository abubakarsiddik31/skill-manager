export type AgentTool = "claude" | "codex" | "cursor" | "gemini" | "opencode";

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

export interface ToolInfo {
  tool: AgentTool;
  label: string;
  skillsDir: string;
  dirExists: boolean;
}

export interface ProjectInfo {
  path: string;
  name: string;
}

export type View = { kind: "global" } | { kind: "project"; project: ProjectInfo };

