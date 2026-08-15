import type { ProjectInfo, ToolEntry, View } from "../types";

const ALL = "all" as const;

interface SidebarProps {
  toolEntries: ToolEntry[];
  totalSkillCount: number;
  countForEntry: (entry: ToolEntry) => number;
  projects: ProjectInfo[];
  view: View;
  activeToolId: string | typeof ALL;
  onSelectAll: () => void;
  onSelectTool: (toolId: string) => void;
  onOpenProject: (project: ProjectInfo) => void;
  onAddProject: () => void;
}

export function Sidebar({
  toolEntries,
  totalSkillCount,
  countForEntry,
  projects,
  view,
  activeToolId,
  onSelectAll,
  onSelectTool,
  onOpenProject,
  onAddProject,
}: SidebarProps) {
  return (
    <aside className="sidebar">
      <div className="brand">
        <img src="/logo.svg" alt="Skill Manager" />
        <span className="brand-name">
          Skill <span className="accent">Manager</span>
        </span>
      </div>

      <div
        className={`nav-item ${view.kind === "global" && activeToolId === ALL ? "active" : ""}`}
        onClick={onSelectAll}
      >
        <span>all skills</span>
        <span className="count">{totalSkillCount}</span>
      </div>

      {toolEntries.map((entry) => {
        const anyDirExists = entry.folders.some((f) => f.dirExists);
        return (
          <div
            key={entry.id}
            className={`nav-item ${view.kind === "global" && activeToolId === entry.id ? "active" : ""}`}
            onClick={() => onSelectTool(entry.id)}
            title={entry.folders.map((f) => f.dir).join("\n")}
          >
            <span className={`nav-label-col ${anyDirExists ? "" : "dir-missing"}`}>
              <span>{entry.label}</span>
              <span className="nav-sub">
                {entry.folders.map((f) => f.dir.replace(/^.*(\/|\\)/, "~…/")).join(" + ")}
              </span>
            </span>
            <span className="count">{countForEntry(entry)}</span>
          </div>
        );
      })}

      <div className="nav-section-label">projects</div>

      {projects.map((p) => (
        <div
          key={p.path}
          className={`nav-item ${view.kind === "project" && view.project.path === p.path ? "active" : ""}`}
          onClick={() => onOpenProject(p)}
          title={p.path}
        >
          <span>{p.name}</span>
        </div>
      ))}

      <div className="nav-item add-project" onClick={onAddProject}>
        <span>+ add project</span>
      </div>

      <div className="sidebar-footer">open source · MIT</div>
    </aside>
  );
}
