import { invoke } from "@tauri-apps/api/core";
import type { Skill, ToolInfo } from "../types";

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
};
