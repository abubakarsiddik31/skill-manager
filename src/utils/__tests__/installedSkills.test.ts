import { describe, expect, it } from "vitest";
import { installedSkillKeys, isRemoteSkillInstalled } from "../installedSkills";
import type { RemoteSkill, Skill } from "../../types";

function skill(overrides: Partial<Skill> = {}): Skill {
  return {
    id: "/root/.claude/skills/pdf/SKILL.md",
    tool: "claude",
    name: "pdf",
    description: "",
    path: "/root/.claude/skills/pdf",
    scope: "user",
    enabled: true,
    ...overrides,
  };
}

function remote(name: string): RemoteSkill {
  return {
    name,
    description: null,
    owner: "anthropics",
    repo: "skills",
    path: `skills/${name}`,
    branch: "main",
  };
}

describe("installedSkillKeys", () => {
  it("lists both the folder and the frontmatter name", () => {
    const keys = installedSkillKeys([skill({ name: "PDF handling" })]);
    expect(keys.has("pdf handling")).toBe(true); // frontmatter name
    expect(keys.has("pdf")).toBe(true); // folder name
  });

  it("matches case-insensitively and ignores surrounding spaces", () => {
    const keys = installedSkillKeys([skill({ name: "  Docx  " })]);
    expect(keys.has("docx")).toBe(true);
    expect(keys.has("DOCX")).toBe(false); // keys are normalized to lowercase
    expect(keys.has("docx ")).toBe(false); // remote names are trimmed before lookup
  });

  it("takes the folder from windows-style paths too", () => {
    const keys = installedSkillKeys([
      skill({ name: "renamed", path: "C:\\Users\\me\\.claude\\skills\\xlsx" }),
    ]);
    expect(keys.has("xlsx")).toBe(true);
  });

  it("works for an empty skills list", () => {
    expect(installedSkillKeys([]).size).toBe(0);
    expect(isRemoteSkillInstalled(remote("pdf"), installedSkillKeys([]))).toBe(false);
  });
});

describe("isRemoteSkillInstalled", () => {
  it("matches a remote skill whose name equals an installed folder", () => {
    const keys = installedSkillKeys([skill()]);
    expect(isRemoteSkillInstalled(remote("pdf"), keys)).toBe(true);
  });

  it("matches through the frontmatter name, not just the folder", () => {
    const keys = installedSkillKeys([skill({ name: "Brand Guidelines" })]);
    expect(isRemoteSkillInstalled(remote("brand-guidelines"), keys)).toBe(false);
    expect(isRemoteSkillInstalled(remote("brand guidelines"), keys)).toBe(true);
  });

  it("does not match unrelated skills", () => {
    const keys = installedSkillKeys([skill()]);
    expect(isRemoteSkillInstalled(remote("docx"), keys)).toBe(false);
  });
});
