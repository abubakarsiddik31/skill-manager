import { useEffect, useState } from "react";
import { api } from "../lib/api";
import type { ProjectInfo } from "../types";

/** Tracks which project folders the user has added - not their skills. */
export function useProjects() {
  const [projects, setProjects] = useState<ProjectInfo[]>([]);

  async function refresh() {
    setProjects(await api.listProjects());
  }

  useEffect(() => {
    refresh();
  }, []);

  async function pickAndAdd(): Promise<ProjectInfo | null> {
    const path = await api.pickProjectFolder();
    if (!path) return null;
    const project = await api.addProject(path);
    await refresh();
    return project;
  }

  async function forget(project: ProjectInfo) {
    await api.removeProject(project.path);
    await refresh();
  }

  return { projects, pickAndAdd, forget };
}
