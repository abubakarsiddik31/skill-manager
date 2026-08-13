import { useEffect, useMemo, useState } from "react";
import { api } from "../lib/api";
import { renderMarkdown } from "../lib/markdown";
import type { Skill } from "../types";

type EditorMode = "view" | "edit";

interface EditorModalProps {
  skill: Skill;
  onClose: () => void;
}

export function EditorModal({ skill, onClose }: EditorModalProps) {
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

  async function save() {
    setSaving(true);
    await api.writeSkillContent(skill.id, content);
    setSaving(false);
    onClose();
  }

  const html = useMemo(() => renderMarkdown(content), [content]);

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <span className="title">{skill.name} / SKILL.md</span>
          <div className="mode-tabs">
            <button
              className={`mode-tab ${mode === "view" ? "active" : ""}`}
              onClick={() => setMode("view")}
            >
              view
            </button>
            <button
              className={`mode-tab ${mode === "edit" ? "active" : ""}`}
              onClick={() => setMode("edit")}
            >
              edit
            </button>
          </div>
          <button className="icon-btn" onClick={onClose}>
            close
          </button>
        </div>

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

        <div className="modal-footer">
          <button className="btn" onClick={onClose}>
            cancel
          </button>
          <button className="btn primary" onClick={save} disabled={loading || saving}>
            {saving ? "saving..." : "save"}
          </button>
        </div>
      </div>
    </div>
  );
}
