import { useEffect, useState } from "react";
import { api } from "../lib/api";
import type { Skill, ToolInfo } from "../types";
import { useSkillMutations } from "./useSkillMutations";

export function useGlobalSkills() {
  const [tools, setTools] = useState<ToolInfo[]>([]);
  const [skills, setSkills] = useState<Skill[]>([]);

  async function refresh() {
    const [t, s] = await Promise.all([api.listTools(), api.listSkills()]);
    setTools(t);
    setSkills(s);
  }

  useEffect(() => {
    refresh();
  }, []);

  const { toggle, remove } = useSkillMutations(setSkills);

  return { tools, skills, refresh, toggle, remove };
}
