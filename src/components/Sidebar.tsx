import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { ProjectInfo, ToolEntry, View } from "../types";

const ALL = "all" as const;
const REPO_URL = "https://github.com/abubakarsiddik31/skill-manager";

function link(url: string) {
  return () => openUrl(url).catch(console.error);
}

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
  const [version, setVersion] = useState("");

  useEffect(() => {
    getVersion().then(setVersion).catch(console.error);
  }, []);

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
            <span className={anyDirExists ? "" : "dir-missing"}>{entry.label}</span>
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

      <div className="sidebar-footer">
        <div className="footer-version">
          skill manager{version ? ` v${version}` : ""}
        </div>
        <div className="footer-links">
          <a onClick={link(REPO_URL)}>github</a>
          <a onClick={link(`${REPO_URL}/issues/new?template=bug_report.yml`)}>report a bug</a>
          <a onClick={link(`${REPO_URL}/blob/main/LICENSE`)}>mit</a>
        </div>
      </div>
    </aside>
  );
}
