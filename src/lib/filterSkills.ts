import type { AgentTool, Skill } from "../types";

/**
 * Filter skills by search query and, optionally, by a set of folders —
 * a tool's view passes every folder it reads, so shared-folder skills
 * appear under each tool that can see them.
 */
export function filterSkills(skills: Skill[], query: string, folders?: Set<AgentTool>): Skill[] {
  const q = query.trim().toLowerCase();
  return skills
    .filter((s) => !folders || folders.has(s.tool))
    .filter((s) => q.length === 0 || (s.name + s.description).toLowerCase().includes(q))
    .sort((a, b) => a.name.localeCompare(b.name));
}
