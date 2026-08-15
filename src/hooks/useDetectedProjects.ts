import { useEffect, useState } from "react";
import { api } from "../lib/api";
import type { DetectedProject } from "../types";

/**
 * Projects the user seems to work in (per detect_projects), kept
 * current as the tracked list changes so the sidebar can suggest the
 * ones not added yet. Null while the first scan runs.
 */
export function useDetectedProjects(trackedPaths: string[]) {
  const [detected, setDetected] = useState<DetectedProject[] | null>(null);
  const key = trackedPaths.join("\n");

  useEffect(() => {
    let cancelled = false;
    api
      .detectProjects(key ? key.split("\n") : [])
      .then((result) => {
        if (!cancelled) setDetected(result);
      })
      .catch(() => {
        if (!cancelled) setDetected([]);
      });
    return () => {
      cancelled = true;
    };
  }, [key]);

  return detected;
}
