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
    // a quick project switch must not let the previous project's slow
    // response overwrite the new one
    let cancelled = false;
    setLoading(true);
    api.listProjectSkills(project.path).then((s) => {
      if (cancelled) return;
      setSkills(s);
      setLoading(false);
    });
    return () => {
      cancelled = true;
    };
  }, [project?.path]);

  const { toggle, remove } = useSkillMutations(setSkills);

  return { skills, loading, toggle, remove };
}
