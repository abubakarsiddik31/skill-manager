import { useEffect, useRef, useState } from "react";
import { api } from "../lib/api";
import type { Skill, ToolEntry } from "../types";
import { useSkillMutations } from "./useSkillMutations";

export function useGlobalSkills() {
  const [toolEntries, setToolEntries] = useState<ToolEntry[]>([]);
  const [skills, setSkills] = useState<Skill[]>([]);
  // only the most recent refresh may commit — overlapping refreshes
  // (mount + a mutation) otherwise race and can restore stale data
  const latestRequest = useRef(0);

  async function refresh() {
    const request = ++latestRequest.current;
    const [entries, s] = await Promise.all([api.listToolEntries(), api.listSkills()]);
    if (request !== latestRequest.current) return;
    setToolEntries(entries);
    setSkills(s);
  }

  useEffect(() => {
    refresh();
  }, []);

  const { toggle, remove } = useSkillMutations(setSkills);

  return { toolEntries, skills, refresh, toggle, remove };
}
