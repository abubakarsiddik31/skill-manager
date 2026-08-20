import type { AgentTool } from "./tool";

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
