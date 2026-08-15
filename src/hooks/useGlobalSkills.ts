import { useEffect, useState } from "react";
import { api } from "../lib/api";
import type { Skill, ToolEntry } from "../types";
import { useSkillMutations } from "./useSkillMutations";

export function useGlobalSkills() {
  const [toolEntries, setToolEntries] = useState<ToolEntry[]>([]);
  const [skills, setSkills] = useState<Skill[]>([]);

  async function refresh() {
    const [entries, s] = await Promise.all([api.listToolEntries(), api.listSkills()]);
    setToolEntries(entries);
    setSkills(s);
  }

  useEffect(() => {
    refresh();
  }, []);

  const { toggle, remove } = useSkillMutations(setSkills);

  return { toolEntries, skills, refresh, toggle, remove };
}
