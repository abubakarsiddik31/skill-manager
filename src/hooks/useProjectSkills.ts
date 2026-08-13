import { useEffect, useState } from "react";
import { api } from "../lib/api";
import type { ProjectInfo, Skill } from "../types";
import { useSkillMutations } from "./useSkillMutations";

/** Loads the skill breakdown for whichever project is currently open. */
export function useProjectSkills(project: ProjectInfo | null) {
  const [skills, setSkills] = useState<Skill[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!project) {
      setSkills([]);
      return;
    }
    setLoading(true);
    api.listProjectSkills(project.path).then((s) => {
      setSkills(s);
      setLoading(false);
    });
  }, [project?.path]);

  const { toggle, remove } = useSkillMutations(setSkills);

  return { skills, loading, toggle, remove };
}
