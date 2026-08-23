import { useMemo, useState } from "react";
import { api } from "../../api";
import { searchRemoteSkills } from "../../utils/collectionSearch";
import { CloseIcon } from "../ui/icons";
import { useCollections } from "../../hooks/useCollections";
import type {
  AgentTool,
  CatalogSource,
  ProjectInfo,
  RemoteSkill,
  Skill,
  ToolEntry,
} from "../../types";

interface BrowseViewProps {
  toolEntries: ToolEntry[];
  projects: ProjectInfo[];
  /** Tool/scope preselected from where the browser was opened. */
  defaultTool?: AgentTool;
  defaultProject: ProjectInfo | null;
  onBack: () => void;
  onInstalled: (skill: Skill) => void;
}

function ownTool(entry: ToolEntry): AgentTool | undefined {
  return entry.folders.find((f) => f.role === "own")?.tool ?? entry.folders[0]?.tool;
}

const SOURCE_LABELS: Record<CatalogSource, string> = {
  manifest: "live catalog",
  cached: "cached catalog",
  bundled: "bundled catalog",
};

/** The full-window collections browser: built-ins are listed from the
 *  index bundled with the app (zero GitHub traffic); install and the
 *  refresh button are the only actions that hit the GitHub API. */
export function BrowseView({
  toolEntries,
  projects,
  defaultTool,
  defaultProject,
  onBack,
  onInstalled,
}: BrowseViewProps) {
  const browseState = useCollections();
  const [query, setQuery] = useState("");
  const [addRepo, setAddRepo] = useState("");
  const [addError, setAddError] = useState<string | null>(null);
  const [installing, setInstalling] = useState<RemoteSkill | null>(null);
  const [busy, setBusy] = useState(false);
  const [confirmingOverwrite, setConfirmingOverwrite] = useState(false);
  const [tool, setTool] = useState<AgentTool>(defaultTool ?? "claude");
  const [scope, setScope] = useState<"user" | "project">(defaultProject ? "project" : "user");
  const [projectPath, setProjectPath] = useState<string>(
    defaultProject?.path ?? projects[0]?.path ?? "",
  );
  const [installError, setInstallError] = useState<string | null>(null);
  const [installedNames, setInstalledNames] = useState<string[]>([]);

  const filtered = useMemo(
    () => searchRemoteSkills(browseState.skills, query),
    [browseState.skills, query],
  );

  const totalSkills = browseState.collections.reduce(
    (sum, c) => sum + (c.skillCount ?? 0),
    0,
  );

  function openPicker(skill: RemoteSkill) {
    setInstallError(null);
    setConfirmingOverwrite(false);
    setInstalling(skill);
  }

  function closePicker() {
    setInstallError(null);
    setConfirmingOverwrite(false);
    setInstalling(null);
    setBusy(false);
  }

  async function install(skill: RemoteSkill, overwrite = false) {
    if (!browseState.activeId) return;
    setBusy(true);
    setInstallError(null);
    try {
      const result = await api.installSkill({
        tool,
        scope,
        projectPath: scope === "project" ? projectPath : undefined,
        skill,
        collectionId: browseState.activeId,
        overwrite,
      });
      setInstalledNames((names) => [...names, skill.name]);
      closePicker();
      onInstalled(result.skill);
    } catch (e) {
      const message = String(e);
      setBusy(false);
      // A collision swaps the picker for an inline overwrite prompt —
      // destructive confirmation stays explicit, but in-window.
      if (!overwrite && message.includes("already exists")) {
        setConfirmingOverwrite(true);
      } else {
        setInstallError(message);
      }
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

  const activeCollection = browseState.collections.find(
    (c) => c.id === browseState.activeId,
  );

  return (
    <div className="browse-view">
      <header className="browse-header">
        <button className="btn" onClick={onBack} title="back to your skills">
          ← back
        </button>
        <div className="topbar-title">
          <h1>browse collections</h1>
          <span className="subtitle">
            {browseState.collections.length} collections · {totalSkills} skills ready to install
          </span>
        </div>
        <span className="source-pill" title="where the collection list comes from">
          {SOURCE_LABELS[browseState.source]}
        </span>
      </header>

      <div className="browse-body">
        <aside className="browse-rail">
          <div className="rail-label">collections</div>
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

        <section className="browse-content">
          <div className="browse-toolbar">
            <input
              className="search"
              placeholder="search skills..."
              value={query}
              onChange={(e) => setQuery(e.target.value)}
            />
            <span className="browse-count">
              {activeCollection ? `${activeCollection.title} — ` : ""}
              {filtered.length} of {browseState.skills.length}
            </span>
            <button
              className="btn"
              onClick={browseState.refreshActive}
              disabled={browseState.loading}
              title="re-enumerate this repo live from GitHub"
            >
              {browseState.loading ? "refreshing…" : "refresh"}
            </button>
          </div>

          {browseState.error && <div className="create-error browse-error">{browseState.error}</div>}

          <div className="skill-grid">
            {filtered.map((skill) => {
              const installed = installedNames.includes(skill.name);
              const isTarget = installing?.name === skill.name && installing.path === skill.path;
              return (
                <article
                  key={`${skill.owner}/${skill.repo}/${skill.path}`}
                  className="remote-skill-card"
                >
                  <div className="remote-skill-name">{skill.name}</div>
                  <p className="remote-skill-desc">
                    {skill.description ?? "no bundled description — install it to read the SKILL.md."}
                  </p>
                  <div className="remote-skill-meta">
                    {skill.path ? `${skill.repo} · ${skill.path}` : `${skill.repo} · repo root`}
                  </div>

                  {isTarget ? (
                    confirmingOverwrite ? (
                      <div className="overwrite-row">
                        <span>already exists in that folder</span>
                        <button
                          className="btn danger"
                          disabled={busy}
                          onClick={() => install(skill, true)}
                        >
                          overwrite
                        </button>
                        <button className="btn" onClick={closePicker}>
                          cancel
                        </button>
                      </div>
                    ) : (
                      <div className="install-controls">
                        <div className="row">
                          <select
                            value={tool}
                            onChange={(e) => setTool(e.target.value as AgentTool)}
                            aria-label="install into tool"
                          >
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
                            aria-label="install scope"
                          >
                            <option value="user">user</option>
                            <option value="project" disabled={projects.length === 0}>
                              project
                            </option>
                          </select>
                        </div>
                        {scope === "project" && !defaultProject && (
                          <select
                            value={projectPath}
                            onChange={(e) => setProjectPath(e.target.value)}
                            aria-label="project"
                          >
                            {projects.map((p) => (
                              <option key={p.path} value={p.path}>
                                {p.name}
                              </option>
                            ))}
                          </select>
                        )}
                        <div className="row">
                          <button className="btn grow" disabled={busy} onClick={() => install(skill)}>
                            {busy ? "installing…" : "install"}
                          </button>
                          <button className="btn" onClick={closePicker}>
                            cancel
                          </button>
                        </div>
                      </div>
                    )
                  ) : (
                    <div className="remote-skill-footer">
                      {installed ? (
                        <span className="installed-badge">installed ✓</span>
                      ) : (
                        <button className="btn grow" onClick={() => openPicker(skill)}>
                          install
                        </button>
                      )}
                    </div>
                  )}

                  {isTarget && installError && <div className="create-error">{installError}</div>}
                </article>
              );
            })}

            {browseState.loading && browseState.skills.length === 0 && (
              <div className="empty-state">loading collection…</div>
            )}
            {!browseState.loading &&
              filtered.length === 0 &&
              !browseState.error &&
              browseState.activeId !== null && (
                <div className="empty-state">
                  no skills {query ? "match " : "in this collection "}
                  {query ? `“${query}”` : "— try refresh to enumerate it from GitHub."}
                </div>
              )}
          </div>
        </section>
      </div>
    </div>
  );
}
