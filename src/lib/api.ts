import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { ProjectInfo, Skill, ToolInfo } from "../types";

/**
 * Thin client over the Rust-side adapters (src-tauri/src/skills).
 * Each supported tool (Claude, Codex, Cursor, OpenCode) implements the
 * same SkillAdapter trait on the backend; this module is the single
 * place the frontend talks to that layer.
 */
export const api = {
  listTools(): Promise<ToolInfo[]> {
    return invoke("list_tools");
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
  addProject(path: string): Promise<ProjectInfo> {
    return invoke("add_project", { path });
  },
  removeProject(path: string): Promise<void> {
    return invoke("remove_project", { path });
  },
  listProjectSkills(path: string): Promise<Skill[]> {
    return invoke("list_project_skills", { path });
  },
  async pickProjectFolder(): Promise<string | null> {
    const selected = await open({ directory: true, multiple: false });
    return typeof selected === "string" ? selected : null;
  },
};
