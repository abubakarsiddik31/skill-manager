import { useEffect, useMemo, useState } from "react";
import { api } from "./lib/api";
import type { AgentTool, Skill, ToolInfo } from "./types";
import "./App.css";

const ALL: AgentTool | "all" = "all";

function EditorModal({
  skill,
  onClose,
}: {
  skill: Skill;
  onClose: () => void;
}) {
  const [content, setContent] = useState("");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

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

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <span className="title">{skill.name} / SKILL.md</span>
          <button className="icon-btn" onClick={onClose}>
            close
          </button>
        </div>
        <textarea
          value={content}
          onChange={(e) => setContent(e.target.value)}
          disabled={loading}
          spellCheck={false}
        />
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

function App() {
  const [tools, setTools] = useState<ToolInfo[]>([]);
  const [skills, setSkills] = useState<Skill[]>([]);
  const [activeTool, setActiveTool] = useState<AgentTool | "all">(ALL);
  const [query, setQuery] = useState("");
  const [editing, setEditing] = useState<Skill | null>(null);

  async function refresh() {
    const [t, s] = await Promise.all([api.listTools(), api.listSkills()]);
    setTools(t);
    setSkills(s);
  }

  useEffect(() => {
    refresh();
  }, []);

  const filtered = useMemo(() => {
    return skills
      .filter((s) => activeTool === ALL || s.tool === activeTool)
      .filter((s) =>
        query.trim().length === 0
          ? true
          : (s.name + s.description).toLowerCase().includes(query.toLowerCase()),
      )
      .sort((a, b) => a.name.localeCompare(b.name));
  }, [skills, activeTool, query]);

  async function toggle(skill: Skill) {
    const updated = await api.setSkillEnabled(skill.id, !skill.enabled);
    setSkills((prev) =>
      prev.map((s) => (s.id === skill.id ? updated : s)),
    );
  }

  async function remove(skill: Skill) {
    if (!confirm(`Delete "${skill.name}"? This removes its folder from disk.`)) return;
    await api.deleteSkill(skill.id);
    setSkills((prev) => prev.filter((s) => s.id !== skill.id));
  }

  const countFor = (tool: AgentTool) => skills.filter((s) => s.tool === tool).length;

  return (
    <div className="app">
      <aside className="sidebar">
        <div className="brand">
          <img src="/logo.svg" alt="Skill Manager" />
          <span className="brand-name">
            skill<span className="accent">manager</span>
          </span>
        </div>

        <div
          className={`nav-item ${activeTool === ALL ? "active" : ""}`}
          onClick={() => setActiveTool(ALL)}
        >
          <span>all skills</span>
          <span className="count">{skills.length}</span>
        </div>

        {tools.map((t) => (
          <div
            key={t.tool}
            className={`nav-item ${activeTool === t.tool ? "active" : ""}`}
            onClick={() => setActiveTool(t.tool)}
            title={t.skillsDir}
          >
            <span className={t.dirExists ? "" : "dir-missing"}>{t.label}</span>
            <span className="count">{countFor(t.tool)}</span>
          </div>
        ))}

        <div className="sidebar-footer">open source · MIT</div>
      </aside>

      <main className="main">
        <div className="topbar">
          <div>
            <h1>
              {activeTool === ALL
                ? "all skills"
                : tools.find((t) => t.tool === activeTool)?.label}
            </h1>
            <span className="subtitle">{filtered.length} shown</span>
          </div>
          <input
            className="search"
            placeholder="search skills..."
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
        </div>

        <div className="skill-list">
          {filtered.length === 0 && (
            <div className="empty-state">
              no skills found. drop a folder with a{" "}
              <code>SKILL.md</code> file into the tool's skills directory
              and it will show up here.
            </div>
          )}

          {filtered.map((skill) => (
            <div
              key={skill.id}
              className={`skill-card ${skill.enabled ? "" : "disabled"}`}
            >
              <div
                className={`toggle ${skill.enabled ? "on" : ""}`}
                onClick={() => toggle(skill)}
                title={skill.enabled ? "disable" : "enable"}
              >
                <div className="knob" />
              </div>

              <div className="skill-main">
                <div className="skill-name-row">
                  <span className="skill-name">{skill.name}</span>
                  <span className="tool-tag">{skill.tool}</span>
                  <span className="scope-tag">{skill.scope}</span>
                </div>
                {skill.description && (
                  <div className="skill-desc">{skill.description}</div>
                )}
                <div className="skill-path">{skill.path}</div>
              </div>

              <div className="skill-actions">
                <button className="icon-btn" onClick={() => setEditing(skill)}>
                  edit
                </button>
                <button className="icon-btn danger" onClick={() => remove(skill)}>
                  delete
                </button>
              </div>
            </div>
          ))}
        </div>
      </main>

      {editing && <EditorModal skill={editing} onClose={() => setEditing(null)} />}
    </div>
  );
}

export default App;
