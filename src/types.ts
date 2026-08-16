export type AgentTool =
  | "claude"
  | "agents"
  | "copilot"
  | "crush"
  | "cursor"
  | "factory"
  | "gemini"
  | "junie"
  | "kiro"
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
  pinned: boolean;
  /** unix seconds of the last open in this app; 0 = never */
  lastOpened: number;
  /** recent opens (unix seconds), pruned server-side to ~30 days */
  opens: number[];
  /** cached after the user explicitly opens this project; absent until then */
  skillCount?: number;
}

/** A folder the user seems to work in, found by scanning tool history
 *  and common dev roots — most recently active first. */
export interface DetectedProject {
  path: string;
  name: string;
  /** unix seconds; 0 = unknown */
  lastActive: number;
  /** deliberately unknown until the user opens the project */
  skillCount?: number;
  sources: string[];
}

export type View = { kind: "global" } | { kind: "project"; project: ProjectInfo };
