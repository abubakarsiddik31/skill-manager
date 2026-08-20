import { useState } from "react";
import { api } from "../../api";
import { CloseIcon } from "../ui/icons";
import { ModalShell } from "../ui/ModalShell";
import type { AgentTool, ProjectInfo, Skill, ToolEntry } from "../../types";

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
  const [attempted, setAttempted] = useState(false);

  const trimmedName = name.trim();
  const trimmedDescription = description.trim();
  // Hints only appear after the user tries to submit with invalid input —
  // the button never goes silent about why nothing happened.
  const nameProblemHint = !trimmedName
    ? "a name is required"
    : !NAME_PATTERN.test(trimmedName) || trimmedName.length > 64
      ? "letters, digits, '_', '-', '.' only; no leading dot; max 64 chars"
      : null;
  const descriptionProblemHint = !trimmedDescription ? "a description is required" : null;
  const nameValid = NAME_PATTERN.test(trimmedName) && trimmedName.length <= 64;
  const canSubmit =
    !submitting && nameValid && trimmedDescription.length > 0 && (scope === "user" || projectPath !== "");

  async function submit() {
    if (!canSubmit) {
      // Explain why nothing happened instead of silently doing nothing.
      setAttempted(true);
      return;
    }
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
    <ModalShell className="create-modal" onClose={onClose}>
        <div className="modal-header">
          <span className="title">new skill</span>
          <button className="icon-btn square" onClick={onClose} title="close">
            <CloseIcon />
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
            {attempted && nameProblemHint && <span className="field-hint">{nameProblemHint}</span>}
          </label>

          <label>
            description
            <textarea
              className="add-search"
              rows={3}
              placeholder={"what the skill does, and when to use it\n(newlines are kept)"}
              value={description}
              onChange={(e) => setDescription(e.target.value)}
            />
            {attempted && descriptionProblemHint && (
              <span className="field-hint">{descriptionProblemHint}</span>
            )}
          </label>

          {error && <div className="create-error">{error}</div>}
        </div>

        <div className="modal-footer">
          <div className="footer-spacer" />
          {/* Stay clickable when input is incomplete: clicking explains what's
              missing instead of doing nothing. Only in-flight submits block. */}
          <button className="btn" onClick={submit} disabled={submitting}>
            {submitting ? "creating…" : "create & edit"}
          </button>
        </div>
    </ModalShell>
  );
}
