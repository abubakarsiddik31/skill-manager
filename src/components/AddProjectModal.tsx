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
  const [loadingCached, setLoadingCached] = useState(true);
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

  useEffect(() => {
    let active = true;
    api.listDetectedProjects(trackedPaths)
      .then((projects) => {
        if (active && projects !== null) setDetected(projects);
      })
      .catch(() => undefined)
      .finally(() => {
        if (active) setLoadingCached(false);
      });
    return () => {
      active = false;
    };
  }, [trackedPaths]);

  const filtered = useMemo(() => {
    if (!detected) return [];
    const q = query.trim().toLowerCase();
    const byQuery = (d: DetectedProject) =>
      !q || d.name.toLowerCase().includes(q) || d.path.toLowerCase().includes(q);
    // backend order is most-recently-active first; skill sort is local
    const list =
      sortBy === "skills"
        ? [...detected].sort(
            (a, b) => (b.skillCount ?? -1) - (a.skillCount ?? -1) || b.lastActive - a.lastActive,
          )
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
    if (scanning) return;
    setScanning(true);
    try {
      // Discovery is explicitly requested and its result is persisted. The
      // next visit uses the saved result rather than scanning protected
      // folders again.
      setDetected(await api.refreshDetectedProjects(trackedPaths));
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
          {loadingCached ? (
            <div className="add-modal-empty">loading saved projects…</div>
          ) : detected === null ? (
            <div className="add-modal-empty discovery-empty">
              <p>Choose a folder yourself, or look for recent projects.</p>
              <p className="discovery-note">
                This checks only normal development folders and project paths from your editor and
                agent history. The result is saved, so reopening this picker does not scan again.
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
                          {d.skillCount === undefined
                            ? "skills checked when opened"
                            : `${d.skillCount} skill${d.skillCount === 1 ? "" : "s"}`} ·{" "}
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
            {detected !== null && (
              <button className="btn" onClick={discover} disabled={scanning || adding !== null}>
                {scanning ? "refreshing…" : "refresh recent projects"}
              </button>
            )}
            <button className="btn" onClick={browse} disabled={adding !== null}>
              browse folders…
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
