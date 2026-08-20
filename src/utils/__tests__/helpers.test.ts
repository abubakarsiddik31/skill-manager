import { describe, expect, it } from "vitest";
import { filterSkills } from "../filterSkills";
import { relativeTime } from "../relativeTime";
import type { AgentTool, Skill } from "../../types";

function skill(name: string, description: string, tool: AgentTool = "claude"): Skill {
  return {
    id: `/skills/${name}/SKILL.md`,
    tool,
    name,
    description,
    path: `/skills/${name}`,
    scope: "user",
    enabled: true,
  };
}

describe("filterSkills", () => {
  const skills = [
    skill("zebra", "runs fast"),
    skill("apple", "a fruit"),
    skill("Banana", "also a fruit, yellow"),
  ];

  it("returns everything sorted by name when the query is empty", () => {
    expect(filterSkills(skills, "").map((s) => s.name)).toEqual(["apple", "Banana", "zebra"]);
  });

  it("matches names and descriptions case-insensitively", () => {
    expect(filterSkills(skills, "YELLOW").map((s) => s.name)).toEqual(["Banana"]);
    expect(filterSkills(skills, "apple").map((s) => s.name)).toEqual(["apple"]);
  });

  it("restricts to the given folders", () => {
    const mixed = [skill("a", "x", "claude"), skill("b", "y", "agents")];
    expect(filterSkills(mixed, "", new Set(["agents"])).map((s) => s.name)).toEqual(["b"]);
  });

  it("combines query and folder filters", () => {
    const mixed = [skill("a", "x", "claude"), skill("b-query", "y", "agents")];
    expect(filterSkills(mixed, "query", new Set(["agents"]))).toHaveLength(1);
  });
});

describe("relativeTime", () => {
  const now = Date.now() / 1000;

  it("labels unknown timestamps", () => {
    expect(relativeTime(0)).toBe("unknown");
  });

  it("formats common buckets", () => {
    expect(relativeTime(now - 30)).toBe("just now");
    expect(relativeTime(now - 5 * 60)).toBe("5m ago");
    expect(relativeTime(now - 3 * 60 * 60)).toBe("3h ago");
    expect(relativeTime(now - 2 * 24 * 60 * 60)).toBe("2d ago");
    expect(relativeTime(now - 40 * 24 * 60 * 60)).toBe("1mo ago");
    expect(relativeTime(now - 400 * 24 * 60 * 60)).toBe("1y ago");
  });
});
