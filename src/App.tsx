import { useEffect, useMemo, useState } from "react";
import { api } from "./lib/api";
import type { AgentTool, ProjectInfo, Skill, ToolInfo } from "./types";
import "./App.css";

const ALL: AgentTool | "all" = "all";

type View = { kind: "global" } | { kind: "project"; project: ProjectInfo };

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

function SkillCard({
  skill,
  onToggle,
  onEdit,
  onDelete,
}: {
  skill: Skill;
  onToggle: (skill: Skill) => void;
  onEdit: (skill: Skill) => void;
  onDelete: (skill: Skill) => void;
}) {
  return (
    <div className={`skill-card ${skill.enabled ? "" : "disabled"}`}>
      <div
        className={`toggle ${skill.enabled ? "on" : ""}`}
        onClick={() => onToggle(skill)}
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
        {skill.description && <div className="skill-desc">{skill.description}</div>}
        <div className="skill-path">{skill.path}</div>
      </div>

      <div className="skill-actions">
        <button className="icon-btn" onClick={() => onEdit(skill)}>
          edit
        </button>
        <button className="icon-btn danger" onClick={() => onDelete(skill)}>
          delete
        </button>
      </div>
    </div>
  );
}

function SkillList({
  skills,
  emptyHint,
  onToggle,
  onEdit,
  onDelete,
}: {
  skills: Skill[];
  emptyHint: string;
  onToggle: (skill: Skill) => void;
  onEdit: (skill: Skill) => void;
  onDelete: (skill: Skill) => void;
}) {
  if (skills.length === 0) {
    return (
      <div className="empty-state">
        {emptyHint} Drop a folder with a <code>SKILL.md</code> file into the
        directory and it will show up here.
      </div>
    );
  }
  return (
    <>
      {skills.map((skill) => (
        <SkillCard key={skill.id} skill={skill} onToggle={onToggle} onEdit={onEdit} onDelete={onDelete} />
      ))}
    </>
  );
}

function App() {
  const [tools, setTools] = useState<ToolInfo[]>([]);
  const [skills, setSkills] = useState<Skill[]>([]);
  const [activeTool, setActiveTool] = useState<AgentTool | "all">(ALL);
  const [query, setQuery] = useState("");
  const [editing, setEditing] = useState<Skill | null>(null);

  const [projects, setProjects] = useState<ProjectInfo[]>([]);
  const [projectSkills, setProjectSkills] = useState<Skill[]>([]);
  const [projectLoading, setProjectLoading] = useState(false);
  const [view, setView] = useState<View>({ kind: "global" });

  async function refreshGlobal() {
    const [t, s] = await Promise.all([api.listTools(), api.listSkills()]);
    setTools(t);
    setSkills(s);
  }

  async function refreshProjects() {
    setProjects(await api.listProjects());
  }

  useEffect(() => {
    refreshGlobal();
    refreshProjects();
  }, []);

  async function openProject(project: ProjectInfo) {
    setView({ kind: "project", project });
    setProjectLoading(true);
    setProjectSkills(await api.listProjectSkills(project.path));
    setProjectLoading(false);
  }

  async function addProject() {
    const path = await api.pickProjectFolder();
    if (!path) return;
    const project = await api.addProject(path);
    await refreshProjects();
    openProject(project);
  }

  async function forgetProject(project: ProjectInfo) {
    await api.removeProject(project.path);
    await refreshProjects();
    if (view.kind === "project" && view.project.path === project.path) {
      setView({ kind: "global" });
    }
  }

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

  const filteredProjectSkills = useMemo(() => {
    return projectSkills
      .filter((s) =>
        query.trim().length === 0
          ? true
          : (s.name + s.description).toLowerCase().includes(query.toLowerCase()),
      )
      .sort((a, b) => a.name.localeCompare(b.name));
  }, [projectSkills, query]);

  async function toggle(skill: Skill) {
    const updated = await api.setSkillEnabled(skill.id, !skill.enabled);
    if (view.kind === "global") {
      setSkills((prev) => prev.map((s) => (s.id === skill.id ? updated : s)));
    } else {
      setProjectSkills((prev) => prev.map((s) => (s.id === skill.id ? updated : s)));
    }
  }

  async function remove(skill: Skill) {
    if (!confirm(`Delete "${skill.name}"? This removes its folder from disk.`)) return;
    await api.deleteSkill(skill.id);
    if (view.kind === "global") {
      setSkills((prev) => prev.filter((s) => s.id !== skill.id));
    } else {
      setProjectSkills((prev) => prev.filter((s) => s.id !== skill.id));
    }
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
          className={`nav-item ${view.kind === "global" && activeTool === ALL ? "active" : ""}`}
          onClick={() => {
            setView({ kind: "global" });
            setActiveTool(ALL);
          }}
        >
          <span>all skills</span>
          <span className="count">{skills.length}</span>
        </div>

        {tools.map((t) => (
          <div
            key={t.tool}
            className={`nav-item ${view.kind === "global" && activeTool === t.tool ? "active" : ""}`}
            onClick={() => {
              setView({ kind: "global" });
              setActiveTool(t.tool);
            }}
            title={t.skillsDir}
          >
            <span className={t.dirExists ? "" : "dir-missing"}>{t.label}</span>
            <span className="count">{countFor(t.tool)}</span>
          </div>
        ))}

        <div className="nav-section-label">projects</div>

        {projects.map((p) => (
          <div
            key={p.path}
            className={`nav-item ${view.kind === "project" && view.project.path === p.path ? "active" : ""}`}
            onClick={() => openProject(p)}
            title={p.path}
          >
            <span>{p.name}</span>
          </div>
        ))}

        <div className="nav-item add-project" onClick={addProject}>
          <span>+ add project</span>
        </div>

        <div className="sidebar-footer">open source · MIT</div>
      </aside>

      <main className="main">
        <div className="topbar">
          <div>
            <h1>
              {view.kind === "global"
                ? activeTool === ALL
                  ? "all skills"
                  : tools.find((t) => t.tool === activeTool)?.label
                : view.project.name}
            </h1>
            <span className="subtitle">
              {view.kind === "global"
                ? `${filtered.length} shown`
                : view.project.path}
            </span>
          </div>
          <input
            className="search"
            placeholder="search skills..."
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          {view.kind === "project" && (
            <button className="icon-btn danger" onClick={() => forgetProject(view.project)}>
              forget project
            </button>
          )}
        </div>

        <div className="skill-list">
          {view.kind === "global" ? (
            <SkillList
              skills={filtered}
              emptyHint="No skills found."
              onToggle={toggle}
              onEdit={setEditing}
              onDelete={remove}
            />
          ) : projectLoading ? (
            <div className="empty-state">loading...</div>
          ) : (
            <SkillList
              skills={filteredProjectSkills}
              emptyHint="No skills found in this project."
              onToggle={toggle}
              onEdit={setEditing}
              onDelete={remove}
            />
          )}
        </div>
      </main>

      {editing && <EditorModal skill={editing} onClose={() => setEditing(null)} />}
    </div>
  );
}

export default App;
