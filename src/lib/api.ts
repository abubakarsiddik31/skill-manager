import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { DetectedProject, ProjectInfo, Skill, ToolEntry } from "../types";

/**
 * Thin client over the Rust-side adapters (src-tauri/src/skills).
 * Each skills folder implements the same SkillAdapter trait on the
 * backend, and tools are modeled as readers of folders
 * (list_tool_entries); this module is the single place the frontend
 * talks to that layer.
 */
export const api = {
  listToolEntries(): Promise<ToolEntry[]> {
    return invoke("list_tool_entries");
  },
  listSkills(): Promise<Skill[]> {
    return invoke("list_skills");
  },
  setSkillEnabled(id: string, enabled: boolean): Promise<Skill> {
    return invoke("set_skill_enabled", { id, enabled });
  },
  deleteSkill(id: string): Promise<void> {
    return invoke("delete_skill", { id });
  },
  readSkillContent(id: string): Promise<string> {
    return invoke("read_skill_content", { id });
  },
  writeSkillContent(id: string, content: string): Promise<void> {
    return invoke("write_skill_content", { id, content });
  },

  listProjects(): Promise<ProjectInfo[]> {
    return invoke("list_projects");
  },
  detectProjects(exclude: string[]): Promise<DetectedProject[]> {
    return invoke("detect_projects", { exclude });
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
  listProjectSkills(path: string): Promise<Skill[]> {
    return invoke("list_project_skills", { path });
  },
  async pickProjectFolder(): Promise<string | null> {
    const selected = await open({ directory: true, multiple: false });
    return typeof selected === "string" ? selected : null;
  },
};
