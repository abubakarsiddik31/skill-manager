import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { DetectedProject, ProjectInfo, Skill } from "../types";

/** Invoke wrappers over the project commands in
 *  src-tauri/src/commands/projects.rs. */
export const projectsApi = {
  listProjects(): Promise<ProjectInfo[]> {
    return invoke("list_projects");
  },
  listDetectedProjects(exclude: string[]): Promise<DetectedProject[] | null> {
    return invoke("list_detected_projects", { exclude });
  },
  refreshDetectedProjects(exclude: string[]): Promise<DetectedProject[]> {
    return invoke("refresh_detected_projects", { exclude });
  },
  addProject(path: string): Promise<ProjectInfo> {
    return invoke("add_project", { path });
  },
  removeProject(path: string): Promise<void> {
    return invoke("remove_project", { path });
  },
  setProjectPinned(path: string, pinned: boolean): Promise<void> {
    return invoke("set_project_pinned", { path, pinned });
  },
  touchProject(path: string): Promise<void> {
    return invoke("touch_project", { path });
  },
  listProjectSkillCounts(): Promise<Record<string, number>> {
    return invoke("list_project_skill_counts").then((rows) =>
      Object.fromEntries(
        (rows as { path: string; count: number }[]).map((r) => [r.path, r.count]),
      ),
    );
  },
  listProjectSkills(path: string): Promise<Skill[]> {
    return invoke("list_project_skills", { path });
  },
  async pickProjectFolder(): Promise<string | null> {
    const selected = await open({ directory: true, multiple: false });
    return typeof selected === "string" ? selected : null;
  },
};
