import { useEffect, useMemo, useState } from "react";
import { api } from "../../api";
import { searchRemoteSkills } from "../../utils/collectionSearch";
import { CloseIcon } from "../ui/icons";
import { ModalShell } from "../ui/ModalShell";
import { useCollections } from "../../hooks/useCollections";
import type {
  AgentTool,
  ProjectInfo,
  RemoteSkill,
  Skill,
  ToolEntry,
} from "../../types";

interface BrowseModalProps {
  toolEntries: ToolEntry[];
  projects: ProjectInfo[];
  /** The project view the modal was opened from, if any. */
  activeProject: ProjectInfo | null;
  defaultTool?: AgentTool;
  onClose: () => void;
  onInstalled: (skill: Skill) => void;
}

function ownTool(entry: ToolEntry): AgentTool | undefined {
  return entry.folders.find((f) => f.role === "own")?.tool ?? entry.folders[0]?.tool;
}

/** Browse GitHub collections and install skills into any managed agent
 *  folder. Descriptions load lazily from each remote SKILL.md. */
export function BrowseModal({
  toolEntries,
  projects,
  activeProject,
  defaultTool,
  onClose,
  onInstalled,
}: BrowseModalProps) {
  const browseState = useCollections();
  const [query, setQuery] = useState("");
  const [addRepo, setAddRepo] = useState("");
  const [addError, setAddError] = useState<string | null>(null);
  const [installing, setInstalling] = useState<RemoteSkill | null>(null);
  const [tool, setTool] = useState<AgentTool>(defaultTool ?? "claude");
  const [scope, setScope] = useState<"user" | "project">(activeProject ? "project" : "user");
  const [projectPath, setProjectPath] = useState<string>(activeProject?.path ?? "");
  const [installError, setInstallError] = useState<string | null>(null);
  const [installedNames, setInstalledNames] = useState<string[]>([]);

  const filtered = useMemo(
    () => searchRemoteSkills(browseState.skills, query),
    [browseState.skills, query],
  );

  // Fill descriptions for the visible, unloaded skills (one pass per
  // skill per session; the hook dedupes and caches).
  useEffect(() => {
    for (const skill of filtered) {
      if (skill.description === null) browseState.describe(skill);
    }
  }, [filtered, browseState.describe]);

  async function install(skill: RemoteSkill) {
    if (!browseState.activeId) return;
    setInstallError(null);
    try {
      const result = await api.installSkill({
        tool,
        scope,
        projectPath: scope === "project" ? projectPath : undefined,
        skill,
        collectionId: browseState.activeId,
      });
      setInstalledNames((names) => [...names, skill.name]);
      setInstalling(null);
      onInstalled(result.skill);
    } catch (e) {
      const message = String(e);
      if (message.includes("already exists")) {
        const overwrite = window.confirm(
          `'${skill.name}' already exists in that folder. Overwrite it?`,
        );
        if (overwrite) {
          try {
            const result = await api.installSkill({
              tool,
              scope,
              projectPath: scope === "project" ? projectPath : undefined,
              skill,
              collectionId: browseState.activeId!,
              overwrite: true,
            });
            setInstalledNames((names) => [...names, skill.name]);
            setInstalling(null);
            onInstalled(result.skill);
            return;
          } catch (retryError) {
            setInstallError(String(retryError));
            return;
          }
        }
        return;
      }
      setInstallError(message);
    }
  }

  async function submitAdd() {
    const repo = addRepo.trim();
    if (!repo) return;
    setAddError(null);
    try {
      await browseState.add(repo);
      setAddRepo("");
    } catch (e) {
      setAddError(String(e));
    }
  }

  return (
    <ModalShell className="browse-modal" onClose={onClose}>
      <div className="modal-header">
        <span className="title">browse collections</span>
        <button className="icon-btn square" onClick={onClose} title="close">
          <CloseIcon />
        </button>
      </div>

      <div className="browse-layout">
        <aside className="collection-pane">
          {browseState.source === "bundled" && (
            <div className="collection-notice">
              built-in fallback list (remote catalog not available yet)
            </div>
          )}
          {browseState.collections.map((collection) => (
            <div key={collection.id} className="collection-row">
              <button
                className={`collection-item ${collection.id === browseState.activeId ? "active" : ""}`}
                onClick={() => browseState.select(collection.id)}
              >
                <span className="collection-title">{collection.title}</span>
                <span className="collection-meta">
                  {collection.repo}
                  {collection.skillCount !== null ? ` · ${collection.skillCount} skills` : ""}
                </span>
              </button>
              {!collection.builtin && (
                <button
                  className="icon-btn square"
                  title="remove collection"
                  onClick={() => browseState.remove(collection.id)}
                >
                  <CloseIcon />
                </button>
              )}
            </div>
          ))}
          <div className="collection-add">
            <input
              className="add-search"
              placeholder="owner/repo"
              value={addRepo}
              spellCheck={false}
              onChange={(e) => setAddRepo(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && submitAdd()}
            />
            <button className="btn" onClick={submitAdd} disabled={!addRepo.trim()}>
              add
            </button>
          </div>
          {addError && <div className="create-error">{addError}</div>}
        </aside>

        <div className="browse-main">
          <div className="browse-toolbar">
            <input
              className="add-search"
              placeholder="search skills..."
              value={query}
              onChange={(e) => setQuery(e.target.value)}
            />
            <button className="btn" onClick={browseState.refreshActive} disabled={browseState.loading}>
              {browseState.loading ? "loading…" : "refresh"}
            </button>
          </div>

          {browseState.error && <div className="create-error">{browseState.error}</div>}

          <div className="skill-grid">
            {filtered.map((skill) => {
              const installed = installedNames.includes(skill.name);
              const isTarget = installing?.name === skill.name;
              return (
                <div key={`${skill.owner}/${skill.repo}/${skill.path}`} className="remote-skill-card">
                  <div className="remote-skill-name">{skill.name}</div>
                  <div className="remote-skill-desc">
                    {skill.description ?? "…"}
                  </div>
                  <div className="remote-skill-meta">{skill.repo}</div>
                  {isTarget ? (
                    <div className="install-row">
                      <select value={tool} onChange={(e) => setTool(e.target.value as AgentTool)}>
                        {toolEntries.map((entry) => {
                          const value = ownTool(entry);
                          return value === undefined ? null : (
                            <option key={entry.id} value={value}>
                              {entry.label}
                            </option>
                          );
                        })}
                      </select>
                      <select
                        value={scope}
                        onChange={(e) => setScope(e.target.value as "user" | "project")}
                      >
                        <option value="user">user</option>
                        <option value="project" disabled={projects.length === 0}>
                          project
                        </option>
                      </select>
                      {scope === "project" && !activeProject && (
                        <select value={projectPath} onChange={(e) => setProjectPath(e.target.value)}>
                          {projects.map((p) => (
                            <option key={p.path} value={p.path}>
                              {p.name}
                            </option>
                          ))}
                        </select>
                      )}
                      <button className="btn" onClick={() => install(skill)}>
                        install
                      </button>
                      <button
                        className="btn"
                        onClick={() => {
                          setInstallError(null);
                          setInstalling(null);
                        }}
                      >
                        cancel
                      </button>
                      {installError && <div className="create-error">{installError}</div>}
                    </div>
                  ) : (
                    <button
                      className="btn"
                      onClick={() => {
                        setInstallError(null);
                        setInstalling(skill);
                      }}
                      disabled={installed}
                    >
                      {installed ? "installed ✓" : "add"}
                    </button>
                  )}
                </div>
              );
            })}
            {!browseState.loading &&
              filtered.length === 0 &&
              !browseState.error &&
              browseState.activeId !== null && (
                <div className="empty-state">no skills in this collection.</div>
              )}
          </div>
        </div>
      </div>
    </ModalShell>
  );
}
