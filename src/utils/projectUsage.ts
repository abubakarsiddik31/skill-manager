import type { ProjectInfo } from "../types";

const WINDOW_SECONDS = 30 * 24 * 60 * 60;

/** Opens inside the rolling 30-day window the backend prunes to. */
export function usageCount(project: ProjectInfo): number {
  const cutoff = Date.now() / 1000 - WINDOW_SECONDS;
  return project.opens.filter((t) => t >= cutoff).length;
}

/** Best "when was this last touched" signal available. */
export function lastUsed(project: ProjectInfo): number {
  return Math.max(project.lastOpened, ...project.opens, 0);
}
