import { useEffect, useState } from "react";
import { api } from "../api";
import type { ProjectInfo, Skill } from "../types";
import { useSkillMutations } from "./useSkillMutations";

/** Loads the skill breakdown for whichever project is currently open. */
export function useProjectSkills(project: ProjectInfo | null) {
  const [skills, setSkills] = useState<Skill[]>([]);
  const [loading, setLoading] = useState(false);
  const [nonce, setNonce] = useState(0);

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
  }, [project?.path, nonce]);

  const { toggle, remove } = useSkillMutations(setSkills);

  /** Re-fetches after an out-of-band mutation (e.g. a skill created in
   *  this project from the new-skill modal). */
  function reload() {
    setNonce((n) => n + 1);
  }

  return { skills, loading, toggle, remove, reload };
}
