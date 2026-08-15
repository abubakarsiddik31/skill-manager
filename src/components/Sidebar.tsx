import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import { lastUsed, usageCount } from "../lib/projectUsage";
import type { ProjectInfo, ToolEntry, View } from "../types";

const ALL = "all" as const;
const REPO_URL = "https://github.com/abubakarsiddik31/skill-manager";
/** Rows shown for tools before the list collapses behind "more". */
const PREVIEW_COUNT = 6;
/** Most-used projects (30-day window) shown before the "more" dialog. */
const MOST_USED_COUNT = 3;
/** Minimum project rows in the sidebar for a fresh, usage-less setup. */
const MIN_PROJECT_ROWS = 3;

function link(url: string) {
  return () => openUrl(url).catch(console.error);
}

function PinIcon() {
  return (
    <svg viewBox="0 0 20 20" width="12" height="12" fill="none" stroke="currentColor" strokeWidth="1.6">
      <path d="M7 3h6l-1 5 3 2v2H5v-2l3-2-1-5z" strokeLinejoin="round" />
      <path d="M10 12v5" strokeLinecap="round" />
    </svg>
  );
}

interface SidebarProps {
  toolEntries: ToolEntry[];
  totalSkillCount: number;
  countForEntry: (entry: ToolEntry) => number;
  pinnedTools: Set<string>;
  onTogglePinTool: (toolId: string) => void;
  projects: ProjectInfo[];
  onTogglePinProject: (project: ProjectInfo) => void;
  onShowAllProjects: () => void;
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
  pinnedTools,
  onTogglePinTool,
  projects,
  onTogglePinProject,
  onShowAllProjects,
  view,
  activeToolId,
  onSelectAll,
  onSelectTool,
  onOpenProject,
  onAddProject,
}: SidebarProps) {
  const [version, setVersion] = useState("");
  const [toolsExpanded, setToolsExpanded] = useState(false);

  useEffect(() => {
    getVersion().then(setVersion).catch(console.error);
  }, []);

  // Pinned tools float to the top; the rest keep registry order.
  const sortedTools = [...toolEntries].sort(
    (a, b) => Number(pinnedTools.has(b.id)) - Number(pinnedTools.has(a.id)),
  );
  const visibleTools = toolsExpanded ? sortedTools : sortedTools.slice(0, PREVIEW_COUNT);
  const hiddenTools = sortedTools.length - visibleTools.length;

  // The sidebar only shows pins plus the most-used projects of the last
  // 30 days (topped up to MIN_PROJECT_ROWS); everything else lives in
  // the all-projects dialog.
  const pinnedProjects = projects.filter((p) => p.pinned);
  const mostUsed = projects
    .filter((p) => !p.pinned)
    .sort((a, b) => usageCount(b) - usageCount(a) || lastUsed(b) - lastUsed(a))
    .slice(0, MOST_USED_COUNT);
  const visibleProjects = [...pinnedProjects, ...mostUsed];
  for (const p of projects) {
    if (visibleProjects.length >= Math.max(pinnedProjects.length, MIN_PROJECT_ROWS)) break;
    if (!visibleProjects.some((v) => v.path === p.path)) visibleProjects.push(p);
  }
  const hiddenProjects = projects.length - visibleProjects.length;

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

      {visibleTools.map((entry) => {
        const anyDirExists = entry.folders.some((f) => f.dirExists);
        return (
          <div
            key={entry.id}
            className={`nav-item ${view.kind === "global" && activeToolId === entry.id ? "active" : ""}`}
            onClick={() => onSelectTool(entry.id)}
            title={entry.folders.map((f) => f.dir).join("\n")}
          >
            <span className={anyDirExists ? "" : "dir-missing"}>{entry.label}</span>
            <span className="nav-right">
              <button
                className={`pin-btn ${pinnedTools.has(entry.id) ? "pinned" : ""}`}
                onClick={(e) => {
                  e.stopPropagation();
                  onTogglePinTool(entry.id);
                }}
                title={pinnedTools.has(entry.id) ? "unpin" : "pin"}
              >
                <PinIcon />
              </button>
              <span className="count">{countForEntry(entry)}</span>
            </span>
          </div>
        );
      })}

      {hiddenTools > 0 && (
        <div className="nav-item expand" onClick={() => setToolsExpanded(true)}>
          <span>+ {hiddenTools} more</span>
        </div>
      )}
      {toolsExpanded && sortedTools.length > PREVIEW_COUNT && (
        <div className="nav-item expand" onClick={() => setToolsExpanded(false)}>
          <span>show less</span>
        </div>
      )}

      <div className="nav-section-label">projects</div>

      {visibleProjects.map((p) => (
        <div
          key={p.path}
          className={`nav-item ${view.kind === "project" && view.project.path === p.path ? "active" : ""}`}
          onClick={() => onOpenProject(p)}
          title={p.path}
        >
          <span>{p.name}</span>
          <span className="nav-right">
            <button
              className={`pin-btn ${p.pinned ? "pinned" : ""}`}
              onClick={(e) => {
                e.stopPropagation();
                onTogglePinProject(p);
              }}
              title={p.pinned ? "unpin" : "pin"}
            >
              <PinIcon />
            </button>
          </span>
        </div>
      ))}

      {hiddenProjects > 0 && (
        <div className="nav-item expand" onClick={onShowAllProjects}>
          <span>+ {hiddenProjects} more</span>
        </div>
      )}

      <div className="nav-item add-project" onClick={onAddProject}>
        <span>+ add project</span>
      </div>

      <div className="sidebar-footer">
        <div className="footer-row">
          <img src="/logo.svg" alt="Skill Manager" />
          <span className="footer-name">
            Skill <span className="accent">Manager</span>
          </span>
          {version && (
            <a
              className="footer-version"
              onClick={link(`${REPO_URL}/releases/latest`)}
              title="release notes"
            >
              v{version}
            </a>
          )}
        </div>
        <div className="footer-links">
          <a onClick={link(REPO_URL)}>github</a>
          <span className="sep">·</span>
          <a onClick={link(`${REPO_URL}/issues/new?template=bug_report.yml`)}>report a bug</a>
          <span className="sep">·</span>
          <a onClick={link(`${REPO_URL}/blob/main/LICENSE`)}>mit</a>
        </div>
      </div>
    </aside>
  );
}
