import type { RemoteSkill, Skill } from "../types";

function key(name: string): string {
  return name.trim().toLowerCase();
}

/** Every name an installed skill answers to: its folder (collection
 *  installs name the folder after the remote skill) and its listed
 *  frontmatter name — either can carry the match.
 */
export function installedSkillKeys(skills: Skill[]): Set<string> {
  const keys = new Set<string>();
  for (const skill of skills) {
    keys.add(key(skill.name));
    const folder = skill.path.split(/[\\/]/).filter(Boolean).pop();
    if (folder) keys.add(key(folder));
  }
  return keys;
}

/** A remote skill counts as installed when a managed skill shares its
 *  name — installing again would write into that same folder anyway. */
export function isRemoteSkillInstalled(skill: RemoteSkill, keys: Set<string>): boolean {
  return keys.has(key(skill.name));
}
