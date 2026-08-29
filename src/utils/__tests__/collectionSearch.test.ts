import { describe, expect, it } from "vitest";
import { searchRemoteSkills } from "../collectionSearch";
import type { RemoteSkill } from "../../types";

function remote(name: string, description: string | null): RemoteSkill {
  return { name, description, owner: "anthropics", repo: "skills", path: `skills/${name}`, branch: "main" };
}

describe("searchRemoteSkills", () => {
  const skills = [
    remote("zebra", null),
    remote("apple", "a fruit skill"),
    remote("Banana", "also yellow"),
  ];

  it("returns everything sorted by name when the query is empty", () => {
    expect(searchRemoteSkills(skills, "").map((s) => s.name)).toEqual(["apple", "Banana", "zebra"]);
  });

  it("matches names case-insensitively", () => {
    expect(searchRemoteSkills(skills, "BAN").map((s) => s.name)).toEqual(["Banana"]);
  });

  it("matches descriptions when loaded", () => {
    expect(searchRemoteSkills(skills, "yellow").map((s) => s.name)).toEqual(["Banana"]);
  });

  it("treats unloaded descriptions as empty", () => {
    // "null" would match zebra if its unloaded description were coerced
    // to the string "null".
    expect(searchRemoteSkills(skills, "null")).toEqual([]);
  });
});
