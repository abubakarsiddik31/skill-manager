import { invoke } from "@tauri-apps/api/core";
import type {
  BrowseResult,
  CollectionInfo,
  InstallSkillInput,
  InstallResult,
  ListCollectionsResult,
} from "../types";

/** Invoke wrappers over the collection commands in
 *  src-tauri/src/commands/collections.rs. */
export const collectionsApi = {
  listCollections(): Promise<ListCollectionsResult> {
    return invoke("list_collections");
  },
  browseCollection(id: string): Promise<BrowseResult> {
    return invoke("browse_collection", { id });
  },
  refreshCollection(id: string): Promise<BrowseResult> {
    return invoke("refresh_collection", { id });
  },
  addCollection(repo: string, title?: string): Promise<CollectionInfo> {
    return invoke("add_collection", { repo, title: title ?? null });
  },
  removeCollection(id: string): Promise<void> {
    return invoke("remove_collection", { id });
  },
  installSkill(input: InstallSkillInput): Promise<InstallResult> {
    return invoke("install_skill", {
      tool: input.tool,
      scope: input.scope,
      projectPath: input.projectPath ?? null,
      skill: input.skill,
      collectionId: input.collectionId,
      overwrite: input.overwrite ?? null,
    });
  },
};
