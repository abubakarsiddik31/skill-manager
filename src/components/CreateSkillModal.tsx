import { useEffect, useState } from "react";
import { api } from "../lib/api";
import type { AgentTool, ProjectInfo, Skill, ToolEntry } from "../types";

interface CreateSkillModalProps {
  toolEntries: ToolEntry[];
  projects: ProjectInfo[];
  /** The project view the modal was opened from, if any — locks the
   *  scope to that project. */
  activeProject: ProjectInfo | null;
  /** Preselects this agent's folder when the modal opens. */
  defaultTool?: AgentTool;
  onClose: () => void;
  onCreated: (skill: Skill) => void;
}

/** Mirrors the backend folder-name policy closely enough for instant
 *  feedback; the Rust side re-validates and remains authoritative. */
const NAME_PATTERN = /^[A-Za-z0-9][A-Za-z0-9._-]*$/;

function ownTool(entry: ToolEntry): AgentTool | undefined {
  return entry.folders.find((f) => f.role === "own")?.tool ?? entry.folders[0]?.tool;
}

/**
 * Starts a new skill: pick the agent folder and scope, name it, describe
 * it — the app creates exactly one `<name>/SKILL.md` with minimal
 * frontmatter and opens it in the editor. No content is invented; the
 * instructions are the user's to write.
 */
export function CreateSkillModal({
  toolEntries,
  projects,
  activeProject,
  defaultTool,
  onClose,
  onCreated,
}: CreateSkillModalProps) {
  const firstUsable = toolEntries.map(ownTool).find((t) => t !== undefined);
  const [tool, setTool] = useState<AgentTool>(defaultTool ?? firstUsable ?? "claude");
  const [scope, setScope] = useState<"user" | "project">(activeProject ? "project" : "user");
  const [projectPath, setProjectPath] = useState<string>(activeProject?.path ?? "");
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  const trimmedName = name.trim();
  const trimmedDescription = description.trim();
  const nameProblem = !trimmedName
    ? null
    : !NAME_PATTERN.test(trimmedName) || trimmedName.length > 64
      ? "letters, digits, '_', '-', '.' only; no leading dot; max 64 chars"
      : null;
  const descriptionProblem =
    trimmedDescription && trimmedDescription.includes("\n")
      ? "description must be a single line"
      : null;
  const canSubmit =
    !submitting &&
    !nameProblem &&
    !descriptionProblem &&
    NAME_PATTERN.test(trimmedName) &&
    trimmedDescription.length > 0 &&
    (scope === "user" || projectPath !== "");

  async function submit() {
    if (!canSubmit) return;
    setSubmitting(true);
    setError(null);
    try {
      const skill = await api.createSkill({
        tool,
        scope,
        projectPath: scope === "project" ? projectPath : undefined,
        name: trimmedName,
        description: trimmedDescription,
      });
      onCreated(skill);
    } catch (e) {
      setError(String(e));
      setSubmitting(false);
    }
  }

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal create-modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <span className="title">new skill</span>
          <button className="icon-btn square" onClick={onClose} title="close">
            <svg viewBox="0 0 20 20" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="1.6">
              <path d="M5 5l10 10M15 5 5 15" strokeLinecap="round" />
            </svg>
          </button>
        </div>

        <div className="create-form">
          <label>
            agent folder
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
          </label>

          <label>
            scope
            {activeProject ? (
              <select value="project" disabled title="opened from a project view">
                <option value="project">project — {activeProject.name}</option>
              </select>
            ) : (
              <select
                value={scope}
                onChange={(e) => setScope(e.target.value as "user" | "project")}
              >
                <option value="user">user — applies everywhere</option>
                <option value="project" disabled={projects.length === 0}>
                  project{projects.length === 0 ? " — track a project first" : ""}
                </option>
              </select>
            )}
          </label>

          {scope === "project" && !activeProject && (
            <label>
              project
              <select value={projectPath} onChange={(e) => setProjectPath(e.target.value)}>
                {projects.map((p) => (
                  <option key={p.path} value={p.path}>
                    {p.name}
                  </option>
                ))}
              </select>
            </label>
          )}

          <label>
            name
            <input
              className="add-search"
              placeholder="my-skill"
              value={name}
              spellCheck={false}
              onChange={(e) => setName(e.target.value)}
              autoFocus
            />
            {nameProblem && <span className="field-hint">{nameProblem}</span>}
          </label>

          <label>
            description
            <textarea
              className="add-search"
              rows={2}
              placeholder="what the skill does, and when to use it"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
            />
            {descriptionProblem && <span className="field-hint">{descriptionProblem}</span>}
          </label>

          {error && <div className="create-error">{error}</div>}
        </div>

        <div className="modal-footer">
          <div className="footer-spacer" />
          <button className="btn" onClick={submit} disabled={!canSubmit}>
            {submitting ? "creating…" : "create & edit"}
          </button>
        </div>
      </div>
    </div>
  );
}
