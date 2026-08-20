import type { ToolFolderInfo } from "../types";

interface TopbarProps {
  title: string;
  subtitle: string;
  /** Folders the selected tool reads — shown as the page header. */
  folders?: ToolFolderInfo[];
  query: string;
  onQueryChange: (query: string) => void;
  onForgetProject?: () => void;
  /** Opens the new-skill flow. */
  onNewSkill?: () => void;
}

export function Topbar({ title, subtitle, folders, query, onQueryChange, onForgetProject, onNewSkill }: TopbarProps) {
  return (
    <div className="topbar">
      <div className="topbar-title">
        <h1>{title}</h1>
        <span className="subtitle">{subtitle}</span>
        {folders && folders.length > 0 && (
          <div className="folder-chips">
            {folders.map((f) => (
              <span
                key={f.tool}
                className={`folder-chip ${f.role === "compat" ? "compat" : ""} ${f.dirExists ? "" : "missing"}`}
                title={f.role === "compat" ? `${f.dir} — compatibility path` : f.dir}
              >
                {f.dir}
              </span>
            ))}
          </div>
        )}
      </div>
      <div className="topbar-actions">
        {onNewSkill && (
          <button className="btn" onClick={onNewSkill} title="create a new skill from a minimal template">
            new skill
          </button>
        )}
        <input
          className="search"
          placeholder="search skills..."
          value={query}
          onChange={(e) => onQueryChange(e.target.value)}
        />
        {onForgetProject && (
          <button className="icon-btn danger" onClick={onForgetProject}>
            forget project
          </button>
        )}
      </div>
    </div>
  );
}
