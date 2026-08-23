import type { Skill, SkillScope } from "./skill";
import type { AgentTool } from "./tool";

/** One installable skill in a remote GitHub collection. */
export interface RemoteSkill {
  name: string;
  /** From the app's bundled index; null when only a live tree
   *  enumeration knows the skill (user-added repos). */
  description: string | null;
  owner: string;
  repo: string;
  /** Folder containing the SKILL.md; "" when the repo root is the skill. */
  path: string;
  branch: string;
}

/** One browsable collection, built-in (manifest) or user-added. */
export interface CollectionInfo {
  id: string;
  title: string;
  owner: string;
  repo: string;
  subpath: string | null;
  builtin: boolean;
  skillCount: number | null;
}

export type CatalogSource = "manifest" | "cached" | "bundled";

export interface ListCollectionsResult {
  collections: CollectionInfo[];
  source: CatalogSource;
}

export interface InstallResult {
  skill: Skill;
  skippedLinks: number;
}

export interface InstallSkillInput {
  tool: AgentTool;
  scope: SkillScope;
  projectPath?: string;
  skill: RemoteSkill;
  collectionId: string;
  overwrite?: boolean;
}
