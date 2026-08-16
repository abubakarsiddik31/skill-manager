import { useEffect, useMemo, useState } from "react";
import { api } from "../lib/api";
import { relativeTime } from "../lib/relativeTime";
import { SortToggle } from "./SortToggle";
import type { DetectedProject, ProjectInfo } from "../types";

interface AddProjectModalProps {
  trackedPaths: string[];
  onClose: () => void;
  onAdd: (path: string) => Promise<ProjectInfo | null>;
  onBrowse: () => Promise<ProjectInfo | null>;
}

/**
 * Project discovery is opt-in because macOS may ask for access to folders
 * recovered from editor and agent history. Manual folder browsing remains
 * available without scanning other locations.
 */
export function AddProjectModal({ trackedPaths, onClose, onAdd, onBrowse }: AddProjectModalProps) {
  const [detected, setDetected] = useState<DetectedProject[] | null>(null);
  const [query, setQuery] = useState("");
  const [sortBy, setSortBy] = useState<"activity" | "skills">("activity");
  const [adding, setAdding] = useState<string | null>(null);
  const [scanning, setScanning] = useState(false);

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
    const byQuery = (d: DetectedProject) =>
      !q || d.name.toLowerCase().includes(q) || d.path.toLowerCase().includes(q);
    // backend order is most-recently-active first; skill sort is local
    const list =
      sortBy === "skills"
        ? [...detected].sort((a, b) => b.skillCount - a.skillCount || b.lastActive - a.lastActive)
        : detected;
    return list.filter(byQuery);
  }, [detected, query, sortBy]);

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

  async function discover() {
    if (scanning || detected !== null) return;
    setScanning(true);
    try {
      // This is deliberately user initiated: detection may inspect folders
      // named in agent history or editor recents, including protected macOS
      // locations such as Desktop and Documents.
      setDetected(await api.detectProjects(trackedPaths));
    } catch {
      setDetected([]);
    } finally {
      setScanning(false);
    }
  }

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal add-modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <span className="title">add project</span>
          <SortToggle
            value={sortBy}
            options={[
              { id: "activity", label: "by activity" },
              { id: "skills", label: "by skills" },
            ]}
            onChange={(id) => setSortBy(id as "activity" | "skills")}
          />
          <button className="icon-btn square" onClick={onClose} title="close">
            <svg viewBox="0 0 20 20" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="1.6">
              <path d="M5 5l10 10M15 5 5 15" strokeLinecap="round" />
            </svg>
          </button>
        </div>

        <div className="add-modal-body">
          {detected === null ? (
            <div className="add-modal-empty discovery-empty">
              <p>Choose a folder yourself, or look for recent projects.</p>
              <p className="discovery-note">
                Finding recent projects can ask macOS for access to folders mentioned in your editor
                and agent history. It never runs until you choose it.
              </p>
              <button className="btn" onClick={discover} disabled={scanning}>
                {scanning ? "finding recent projects…" : "find recent projects"}
              </button>
            </div>
          ) : (
            <>
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
            </>
          )}
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
