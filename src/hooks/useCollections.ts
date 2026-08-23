import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "../api";
import type { CatalogSource, CollectionInfo, RemoteSkill } from "../types";

/** Collection browsing state: the catalog list and the skills of the
 *  active collection. Built-ins are served from the bundled index —
 *  descriptions arrive with the listing and no per-skill fetching
 *  happens here (the GitHub API is only spent on install/refresh). */
export function useCollections() {
  const [collections, setCollections] = useState<CollectionInfo[]>([]);
  const [source, setSource] = useState<CatalogSource>("bundled");
  const [activeId, setActiveId] = useState<string | null>(null);
  const [skills, setSkills] = useState<RemoteSkill[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const latestRequest = useRef(0);

  const loadCollections = useCallback(async () => {
    try {
      const result = await api.listCollections();
      setCollections(result.collections);
      setSource(result.source);
      setActiveId((current) => current ?? result.collections[0]?.id ?? null);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const browse = useCallback(async (id: string, force: boolean) => {
    const request = ++latestRequest.current;
    setLoading(true);
    setError(null);
    try {
      const result = force ? await api.refreshCollection(id) : await api.browseCollection(id);
      if (request !== latestRequest.current) return;
      setSkills(result);
    } catch (e) {
      if (request === latestRequest.current) setError(String(e));
    } finally {
      if (request === latestRequest.current) setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadCollections();
  }, [loadCollections]);

  useEffect(() => {
    if (activeId) browse(activeId, false);
  }, [activeId, browse]);

  const select = useCallback((id: string) => {
    setActiveId(id);
  }, []);

  const refreshActive = useCallback(() => {
    if (activeId) browse(activeId, true);
  }, [activeId, browse]);

  const add = useCallback(
    async (repo: string, title?: string) => {
      const added = await api.addCollection(repo, title);
      await loadCollections();
      setActiveId(added.id);
      return added;
    },
    [loadCollections],
  );

  const remove = useCallback(
    async (id: string) => {
      await api.removeCollection(id);
      if (activeId === id) {
        setSkills([]);
        setActiveId(null);
      }
      await loadCollections();
      setActiveId((current) => current ?? null);
    },
    [activeId, loadCollections],
  );

  return {
    collections,
    source,
    activeId,
    skills,
    loading,
    error,
    select,
    refreshActive,
    add,
    remove,
  };
}
