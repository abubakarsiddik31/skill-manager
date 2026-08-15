import { useEffect, useState } from "react";
import { api } from "../lib/api";
import type { ProjectInfo } from "../types";

/**
 * Tracks which project folders the user has added — not their skills —
 * plus a skill count per project so the sidebar can badge rows.
 */
export function useProjects() {
  const [projects, setProjects] = useState<ProjectInfo[]>([]);
  const [skillCounts, setSkillCounts] = useState<Record<string, number>>({});

  async function refresh() {
    setProjects(await api.listProjects());
    setSkillCounts(await api.listProjectSkillCounts());
  }

  useEffect(() => {
    refresh();
  }, []);

  async function add(path: string): Promise<ProjectInfo | null> {
    try {
      const project = await api.addProject(path);
      await refresh();
      return project;
    } catch (err) {
      console.error(err);
      return null;
    }
  }

  async function pickAndAdd(): Promise<ProjectInfo | null> {
    const path = await api.pickProjectFolder();
    if (!path) return null;
    return add(path);
  }

  async function forget(project: ProjectInfo) {
    await api.removeProject(project.path);
    await refresh();
  }

  async function togglePin(project: ProjectInfo) {
    await api.setProjectPinned(project.path, !project.pinned);
    await refresh();
  }

  async function touch(project: ProjectInfo) {
    await api.touchProject(project.path);
    await refresh();
  }

  return { projects, skillCounts, add, pickAndAdd, forget, togglePin, touch, refresh };
}
