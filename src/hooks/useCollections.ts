import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "../api";
import type {
  CatalogSource,
  CollectionInfo,
  RemoteSkill,
  SkillManifest,
} from "../types";

/** Collection browsing state: the catalog list, the skills of the
 *  active collection, and lazy SKILL.md metadata with in-memory
 *  caching (one fetch per skill per session, deduped in-flight). */
export function useCollections() {
  const [collections, setCollections] = useState<CollectionInfo[]>([]);
  const [source, setSource] = useState<CatalogSource>("bundled");
  const [activeId, setActiveId] = useState<string | null>(null);
  const [skills, setSkills] = useState<RemoteSkill[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const latestRequest = useRef(0);
  const manifests = useRef(new Map<string, SkillManifest>());
  const inFlight = useRef(new Set<string>());

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

  /** Patch the description onto the matching skill in state — matched by
   *  reference OR by the owner/repo/path key, so fresh objects from a
   *  re-browse get patched just like the object that triggered the fetch. */
  const patchDescription = useCallback((skill: RemoteSkill, description: string | null) => {
    setSkills((current) =>
      current.map((s) =>
        s === skill || (s.owner === skill.owner && s.repo === skill.repo && s.path === skill.path)
          ? { ...s, description }
          : s,
      ),
    );
  }, []);

  /** Lazily fill one skill's description; resolves null while loading
   *  or on failure (the card keeps its name-only rendering). */
  const describe = useCallback(
    async (skill: RemoteSkill): Promise<SkillManifest | null> => {
      const key = `${skill.owner}/${skill.repo}/${skill.path}`;
      const cached = manifests.current.get(key);
      if (cached) {
        patchDescription(skill, cached.description);
        return cached;
      }
      if (inFlight.current.has(key)) return null;
      inFlight.current.add(key);
      try {
        const manifest = await api.fetchSkillManifest(skill);
        manifests.current.set(key, manifest);
        patchDescription(skill, manifest.description);
        return manifest;
      } catch {
        return null;
      } finally {
        inFlight.current.delete(key);
      }
    },
    [patchDescription],
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
    describe,
  };
}
