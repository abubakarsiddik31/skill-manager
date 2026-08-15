import { useEffect, useMemo, useState } from "react";
import { lastUsed, usageCount } from "../lib/projectUsage";
import { relativeTime } from "../lib/relativeTime";
import type { ProjectInfo } from "../types";

interface ProjectsModalProps {
  projects: ProjectInfo[];
  activePath?: string;
  onClose: () => void;
  onOpen: (project: ProjectInfo) => void;
}

/** The full tracked-project list behind the sidebar's short default. */
export function ProjectsModal({ projects, activePath, onClose, onOpen }: ProjectsModalProps) {
  const [query, setQuery] = useState("");

  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  const sorted = useMemo(
    () =>
      [...projects].sort(
        (a, b) =>
          usageCount(b) - usageCount(a) || lastUsed(b) - lastUsed(a),
      ),
    [projects],
  );

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return sorted;
    return sorted.filter(
      (p) => p.name.toLowerCase().includes(q) || p.path.toLowerCase().includes(q),
    );
  }, [sorted, query]);

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal add-modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <span className="title">projects · {projects.length}</span>
          <button className="icon-btn square" onClick={onClose} title="close">
            <svg viewBox="0 0 20 20" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="1.6">
              <path d="M5 5l10 10M15 5 5 15" strokeLinecap="round" />
            </svg>
          </button>
        </div>

        <div className="add-modal-body">
          <input
            className="add-search"
            placeholder="filter projects…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            autoFocus
            spellCheck={false}
          />

          <div className="detected-list">
            {filtered.length === 0 ? (
              <div className="add-modal-empty">
                {projects.length === 0
                  ? "no projects yet — add one from the sidebar"
                  : `nothing matches "${query.trim()}"`}
              </div>
            ) : (
              filtered.map((p) => {
                const uses = usageCount(p);
                return (
                  <div
                    key={p.path}
                    className={`detected-row ${p.path === activePath ? "active" : ""}`}
                    onClick={() => {
                      onOpen(p);
                      onClose();
                    }}
                    title={p.path}
                  >
                    <div className="row-main">
                      <span className="row-name">{p.name}</span>
                      <span className="row-time">
                        {uses > 0 ? `${uses} open${uses === 1 ? "" : "s"} · ` : ""}
                        {relativeTime(lastUsed(p))}
                      </span>
                    </div>
                    <div className="row-sub">
                      <span className="row-path">{p.path}</span>
                      {p.pinned && <span className="row-sources">pinned</span>}
                    </div>
                  </div>
                );
              })
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
