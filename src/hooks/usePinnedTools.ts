import { useCallback, useState } from "react";

const STORAGE_KEY = "skill-manager:pinned-tools";

/**
 * Tool pins are a pure UI preference — the tool registry itself is
 * static on the backend — so they live in localStorage.
 */
export function usePinnedTools() {
  const [pinned, setPinned] = useState<Set<string>>(() => {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      return new Set(raw ? (JSON.parse(raw) as string[]) : []);
    } catch {
      return new Set();
    }
  });

  const toggle = useCallback((toolId: string) => {
    setPinned((prev) => {
      const next = new Set(prev);
      if (next.has(toolId)) {
        next.delete(toolId);
      } else {
        next.add(toolId);
      }
      try {
        localStorage.setItem(STORAGE_KEY, JSON.stringify([...next]));
      } catch {
        // private mode / storage disabled — pins just won't persist
      }
      return next;
    });
  }, []);

  return { pinned, toggle };
}
