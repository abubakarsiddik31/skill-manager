import { describe, expect, it } from "vitest";
import { orderToolsByPin, selectVisibleProjects } from "../sidebarLists";
import type { ProjectInfo, ToolEntry } from "../../types";

const now = Date.now() / 1000;

function project(path: string, overrides: Partial<ProjectInfo> = {}): ProjectInfo {
  return { path, name: path, pinned: false, lastOpened: 0, opens: [], ...overrides };
}

function tool(id: string): ToolEntry {
  return { id, label: id, folders: [] };
}

describe("orderToolsByPin", () => {
  it("floats pinned tools to the top, keeping registry order otherwise", () => {
    const tools = [tool("claude-code"), tool("codex"), tool("cursor"), tool("gemini")];
    const ordered = orderToolsByPin(tools, new Set(["cursor", "gemini"]));
    expect(ordered.map((t) => t.id)).toEqual(["cursor", "gemini", "claude-code", "codex"]);
  });

  it("does not mutate the input", () => {
    const tools = [tool("a"), tool("b")];
    orderToolsByPin(tools, new Set(["b"]));
    expect(tools.map((t) => t.id)).toEqual(["a", "b"]);
  });
});

describe("selectVisibleProjects", () => {
  it("shows pins first, then the most-used, and hides the rest", () => {
    const projects = [
      project("/a", { pinned: true }),
      project("/b", { opens: [now - 60, now - 120, now - 180] }),
      project("/c", { opens: [now - 60] }),
      project("/d", { opens: [now - 60, now - 120] }),
      project("/e"),
    ];
    const { visible, hiddenCount } = selectVisibleProjects(projects, 3, 3);
    expect(visible.map((p) => p.path)).toEqual(["/a", "/b", "/d", "/c"]);
    expect(hiddenCount).toBe(1);
  });

  it("tops up to minRows when there is no usage history", () => {
    const projects = [project("/a"), project("/b"), project("/c"), project("/d")];
    const { visible, hiddenCount } = selectVisibleProjects(projects, 3, 3);
    expect(visible.map((p) => p.path)).toEqual(["/a", "/b", "/c"]);
    expect(hiddenCount).toBe(1);
  });

  it("keeps every pin visible even beyond minRows", () => {
    const projects = Array.from({ length: 6 }, (_, i) =>
      project(`/p${i}`, { pinned: i < 5 }),
    );
    const { visible, hiddenCount } = selectVisibleProjects(projects, 3, 3);
    expect(visible.filter((p) => p.pinned)).toHaveLength(5);
    expect(visible).toHaveLength(6); // the unpinned project tops the list up
    expect(hiddenCount).toBe(0);
  });

  it("breaks usage ties by most recent use", () => {
    const projects = [
      project("/older", { opens: [now - 5_000] }),
      project("/recent", { opens: [now - 10] }),
    ];
    const { visible } = selectVisibleProjects(projects, 3, 3);
    expect(visible.map((p) => p.path)).toEqual(["/recent", "/older"]);
  });

  it("ignores opens outside the 30-day window", () => {
    const monthAgo = now - 31 * 24 * 60 * 60;
    const projects = [
      project("/stale", { opens: [monthAgo, monthAgo - 60], lastOpened: 0 }),
      project("/fresh", { opens: [now - 60] }),
    ];
    const { visible } = selectVisibleProjects(projects, 3, 3);
    expect(visible[0].path).toBe("/fresh");
  });
});
