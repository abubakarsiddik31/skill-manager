import type { AgentTool, ProjectInfo, ToolInfo, View } from "../types";

const ALL = "all" as const;

interface SidebarProps {
  tools: ToolInfo[];
  totalSkillCount: number;
  countForTool: (tool: AgentTool) => number;
  projects: ProjectInfo[];
  view: View;
  activeTool: AgentTool | "all";
  onSelectAll: () => void;
  onSelectTool: (tool: AgentTool) => void;
  onOpenProject: (project: ProjectInfo) => void;
  onAddProject: () => void;
}

export function Sidebar({
  tools,
  totalSkillCount,
  countForTool,
  projects,
  view,
  activeTool,
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
        className={`nav-item ${view.kind === "global" && activeTool === ALL ? "active" : ""}`}
        onClick={onSelectAll}
      >
        <span>all skills</span>
        <span className="count">{totalSkillCount}</span>
      </div>

      {tools.map((t) => (
        <div
          key={t.tool}
          className={`nav-item ${view.kind === "global" && activeTool === t.tool ? "active" : ""}`}
          onClick={() => onSelectTool(t.tool)}
          title={t.skillsDir}
        >
          <span className={t.dirExists ? "" : "dir-missing"}>{t.label}</span>
          <span className="count">{countForTool(t.tool)}</span>
        </div>
      ))}

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
