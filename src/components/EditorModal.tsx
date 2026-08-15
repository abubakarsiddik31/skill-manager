import { useEffect, useMemo, useState } from "react";
import { api } from "../lib/api";
import { renderMarkdown } from "../lib/markdown";
import type { Skill, ToolEntry } from "../types";

type EditorMode = "view" | "edit";

interface EditorModalProps {
  skill: Skill;
  toolEntries: ToolEntry[];
  onClose: () => void;
  onDelete: (skill: Skill) => void;
}

export function EditorModal({ skill, toolEntries, onClose, onDelete }: EditorModalProps) {
  const [content, setContent] = useState("");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [mode, setMode] = useState<EditorMode>("view");

  useEffect(() => {
    api.readSkillContent(skill.id).then((text) => {
      setContent(text);
      setLoading(false);
    });
  }, [skill.id]);

  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  async function save() {
    setSaving(true);
    await api.writeSkillContent(skill.id, content);
    setSaving(false);
    setMode("view");
  }

  function remove() {
    onDelete(skill);
    onClose();
  }

  const html = useMemo(() => renderMarkdown(content), [content]);

  // Tools that read this skill's folder — toggling or deleting affects
  // all of them, since the folder holds one copy on disk.
  const seers = toolEntries.filter((t) => t.folders.some((f) => f.tool === skill.tool));

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <span className="title">{skill.name} / SKILL.md</span>
          <button
            className={`icon-btn square ${mode === "edit" ? "active" : ""}`}
            onClick={() => setMode(mode === "edit" ? "view" : "edit")}
            title={mode === "edit" ? "view" : "edit"}
          >
            <svg viewBox="0 0 20 20" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="1.6">
              <path d="M13.5 3.5 16.5 6.5 6.5 16.5 3 17 3.5 13.5Z" strokeLinejoin="round" />
            </svg>
          </button>
          <button className="icon-btn square danger" onClick={remove} title="delete">
            <svg viewBox="0 0 20 20" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="1.6">
              <path d="M4 5.5h12M8 5.5V4a1 1 0 0 1 1-1h2a1 1 0 0 1 1 1v1.5M14.5 5.5 14 16a1 1 0 0 1-1 1H7a1 1 0 0 1-1-1L5.5 5.5" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
          </button>
        </div>

        {mode === "view" && (
          <div className="readers-box">
            <div className="readers-head">
              read by {seers.length} tool{seers.length === 1 ? "" : "s"}
            </div>
            <div className="readers-chips">
              {seers.map((t) => {
                const via = t.folders.find((f) => f.tool === skill.tool);
                return (
                  <span
                    key={t.id}
                    className={`chip ${via?.role === "compat" ? "compat" : ""}`}
                    title={via?.role === "compat" ? `via the shared ${skill.tool} folder` : "primary location"}
                  >
                    {t.label}
                  </span>
                );
              })}
            </div>
            {seers.length > 1 && (
              <div className="readers-warn">
                one copy on disk — disabling or deleting affects all {seers.length} tools
              </div>
            )}
          </div>
        )}

        {mode === "edit" ? (
          <textarea
            value={content}
            onChange={(e) => setContent(e.target.value)}
            disabled={loading}
            spellCheck={false}
          />
        ) : (
          <div
            className="markdown-body"
            dangerouslySetInnerHTML={{ __html: loading ? "" : html }}
          />
        )}

        {mode === "edit" && (
          <div className="modal-footer">
            <button className="btn" onClick={() => setMode("view")}>
              cancel
            </button>
            <button className="btn primary" onClick={save} disabled={loading || saving}>
              {saving ? "saving..." : "save"}
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
