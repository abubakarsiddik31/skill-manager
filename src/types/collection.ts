import type { Skill, SkillScope } from "./skill";
import type { AgentTool } from "./tool";

/** One installable skill in a remote GitHub collection. */
export interface RemoteSkill {
  name: string;
  /** Filled lazily from the remote SKILL.md via fetchSkillManifest. */
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

/** Frontmatter read from a remote SKILL.md. */
export interface SkillManifest {
  name: string;
  description: string | null;
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
