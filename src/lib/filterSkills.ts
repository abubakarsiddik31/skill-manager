import type { AgentTool, Skill } from "../types";

export function filterSkills(skills: Skill[], query: string, tool?: AgentTool | "all"): Skill[] {
  const q = query.trim().toLowerCase();
  return skills
    .filter((s) => !tool || tool === "all" || s.tool === tool)
    .filter((s) => q.length === 0 || (s.name + s.description).toLowerCase().includes(q))
    .sort((a, b) => a.name.localeCompare(b.name));
}
