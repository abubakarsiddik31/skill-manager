import type { RemoteSkill } from "../types";

/** Case-insensitive substring search over name and (when loaded)
 *  description; results sorted by name. Unloaded descriptions simply
 *  don't match — the grid fills them lazily. */
export function searchRemoteSkills(skills: RemoteSkill[], query: string): RemoteSkill[] {
  const q = query.trim().toLowerCase();
  const filtered = q
    ? skills.filter(
        (s) =>
          s.name.toLowerCase().includes(q) ||
          (s.description ?? "").toLowerCase().includes(q),
      )
    : skills;
  return [...filtered].sort((a, b) => a.name.localeCompare(b.name));
}
