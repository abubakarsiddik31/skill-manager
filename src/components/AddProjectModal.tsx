import { useEffect, useMemo, useState } from "react";
import { api } from "../lib/api";
import { relativeTime } from "../lib/relativeTime";
import type { DetectedProject, ProjectInfo } from "../types";

interface AddProjectModalProps {
  trackedPaths: string[];
  onClose: () => void;
  onAdd: (path: string) => Promise<ProjectInfo | null>;
  onBrowse: () => Promise<ProjectInfo | null>;
}

/**
 * Suggests folders the user already works in (detected on the backend,
 * latest activity first) with a filter on top, so most adds are one
 * click. Manual folder browsing stays available at the bottom.
 */
export function AddProjectModal({ trackedPaths, onClose, onAdd, onBrowse }: AddProjectModalProps) {
  const [detected, setDetected] = useState<DetectedProject[] | null>(null);
  const [query, setQuery] = useState("");
  const [adding, setAdding] = useState<string | null>(null);

  useEffect(() => {
    // trackedPaths is a snapshot for this modal session
    api.detectProjects(trackedPaths).then(setDetected).catch(() => setDetected([]));
  }, []);

  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  const filtered = useMemo(() => {
    if (!detected) return [];
    const q = query.trim().toLowerCase();
    if (!q) return detected;
    return detected.filter(
      (d) => d.name.toLowerCase().includes(q) || d.path.toLowerCase().includes(q),
    );
  }, [detected, query]);

  async function add(path: string) {
    if (adding) return;
    setAdding(path);
    const project = await onAdd(path);
    setAdding(null);
    if (project) onClose();
  }

  async function browse() {
    const project = await onBrowse();
    if (project) onClose();
  }

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal add-modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <span className="title">add project</span>
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
            {detected === null ? (
              <div className="add-modal-empty">scanning…</div>
            ) : filtered.length === 0 ? (
              <div className="add-modal-empty">
                {detected.length === 0
                  ? "no projects detected — browse for a folder below"
                  : `nothing matches "${query.trim()}"`}
              </div>
            ) : (
              filtered.map((d) => (
                <div
                  key={d.path}
                  className="detected-row"
                  onClick={() => add(d.path)}
                  title={d.path}
                >
                  <div className="row-main">
                    <span className="row-name">{d.name}</span>
                    <span className="row-time">
                      {d.skillCount} skill{d.skillCount === 1 ? "" : "s"} ·{" "}
                      {relativeTime(d.lastActive)}
                    </span>
                  </div>
                  <div className="row-sub">
                    <span className="row-path">{d.path}</span>
                    <span className="row-sources">{d.sources.join(" · ")}</span>
                  </div>
                </div>
              ))
            )}
          </div>
        </div>

        <div className="modal-footer">
          <div className="footer-spacer">
            <button className="btn" onClick={browse} disabled={adding !== null}>
              browse folders…
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
