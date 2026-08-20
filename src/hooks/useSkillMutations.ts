import type { Dispatch, SetStateAction } from "react";
import { api } from "../api";
import type { Skill } from "../types";

/**
 * Toggle/delete work identically for a skill regardless of which list it's
 * displayed in (global vs. a project's) - only the setter differs.
 */
export function useSkillMutations(setSkills: Dispatch<SetStateAction<Skill[]>>) {
  async function toggle(skill: Skill) {
    const updated = await api.setSkillEnabled(skill.id, !skill.enabled);
    setSkills((prev) => prev.map((s) => (s.id === skill.id ? updated : s)));
  }

  async function remove(skill: Skill) {
    if (!confirm(`Delete "${skill.name}"? This removes its folder from disk.`)) return;
    await api.deleteSkill(skill.id);
    setSkills((prev) => prev.filter((s) => s.id !== skill.id));
  }

  return { toggle, remove };
}
