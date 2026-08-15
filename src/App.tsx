import { useEffect, useMemo, useRef, useState } from "react";
import { AddProjectModal } from "./components/AddProjectModal";
import { EditorModal } from "./components/EditorModal";
import { Sidebar } from "./components/Sidebar";
import { SkillList } from "./components/SkillList";
import { Topbar } from "./components/Topbar";
import { useGlobalSkills } from "./hooks/useGlobalSkills";
import { useProjects } from "./hooks/useProjects";
import { useProjectSkills } from "./hooks/useProjectSkills";
import { filterSkills } from "./lib/filterSkills";
import type { ProjectInfo, Skill, ToolEntry, View } from "./types";
import "./App.css";

const ALL = "all" as const;

function App() {
  const [view, setView] = useState<View>({ kind: "global" });
  const [activeToolId, setActiveToolId] = useState<string | typeof ALL>(ALL);
  const [query, setQuery] = useState("");
  const [editing, setEditing] = useState<Skill | null>(null);
  const [addingProject, setAddingProject] = useState(false);
  const skillListRef = useRef<HTMLDivElement>(null);

  const global = useGlobalSkills();
  const projects = useProjects();
  const activeProject = view.kind === "project" ? view.project : null;
  const projectView = useProjectSkills(activeProject);

  const activeTool =
    activeToolId === ALL
      ? null
      : global.toolEntries.find((t) => t.id === activeToolId) ?? null;

  useEffect(() => {
    skillListRef.current?.scrollTo(0, 0);
  }, [view, activeToolId]);

  function selectAll() {
    setView({ kind: "global" });
    setActiveToolId(ALL);
  }

  function selectTool(toolId: string) {
    setView({ kind: "global" });
    setActiveToolId(toolId);
  }

  function openProject(project: ProjectInfo) {
    setView({ kind: "project", project });
  }

  async function addDetectedProject(path: string) {
    const project = await projects.add(path);
    if (project) openProject(project);
    return project;
  }

  async function browseAndAddProject() {
    const project = await projects.pickAndAdd();
    if (project) openProject(project);
    return project;
  }

  async function forgetProject(project: ProjectInfo) {
    await projects.forget(project);
    if (activeProject?.path === project.path) setView({ kind: "global" });
  }

  // A tool's view is the union of every skills folder it reads — a skill
  // in the shared ~/.agents folder correctly shows under Codex, Goose,
  // Amp, and every tool that scans it.
  const folderFilter = useMemo(
    () => (activeTool ? new Set(activeTool.folders.map((f) => f.tool)) : undefined),
    [activeTool],
  );

  const filteredGlobal = useMemo(
    () => filterSkills(global.skills, query, folderFilter),
    [global.skills, query, folderFilter],
  );

  const filteredProjectSkills = useMemo(
    () => filterSkills(projectView.skills, query),
    [projectView.skills, query],
  );

  const countForEntry = (entry: ToolEntry) =>
    global.skills.filter((s) => entry.folders.some((f) => f.tool === s.tool)).length;

  const title =
    view.kind === "global" ? (activeTool ? activeTool.label : "all skills") : view.project.name;

  const subtitle =
    view.kind === "global" ? `${filteredGlobal.length} shown` : view.project.path;

  return (
    <div className="app">
      <Sidebar
        toolEntries={global.toolEntries}
        totalSkillCount={global.skills.length}
        countForEntry={countForEntry}
        projects={projects.projects}
        view={view}
        activeToolId={activeToolId}
        onSelectAll={selectAll}
        onSelectTool={selectTool}
        onOpenProject={openProject}
        onAddProject={() => setAddingProject(true)}
      />

      <main className="main">
        <Topbar
          title={title}
          subtitle={subtitle}
          folders={view.kind === "global" ? activeTool?.folders : undefined}
          query={query}
          onQueryChange={setQuery}
          onForgetProject={view.kind === "project" ? () => forgetProject(view.project) : undefined}
        />

        <div className="skill-list" ref={skillListRef}>
          {view.kind === "global" ? (
            <SkillList
              skills={filteredGlobal}
              toolEntries={global.toolEntries}
              emptyHint="No skills found."
              onToggle={global.toggle}
              onOpen={setEditing}
            />
          ) : projectView.loading ? (
            <div className="empty-state">loading...</div>
          ) : (
            <SkillList
              skills={filteredProjectSkills}
              toolEntries={global.toolEntries}
              emptyHint="No skills found in this project."
              onToggle={projectView.toggle}
              onOpen={setEditing}
            />
          )}
        </div>
      </main>

      {editing && (
        <EditorModal
          skill={editing}
          toolEntries={global.toolEntries}
          onClose={() => setEditing(null)}
          onDelete={view.kind === "global" ? global.remove : projectView.remove}
        />
      )}

      {addingProject && (
        <AddProjectModal
          trackedPaths={projects.projects.map((p) => p.path)}
          onClose={() => setAddingProject(false)}
          onAdd={addDetectedProject}
          onBrowse={browseAndAddProject}
        />
      )}
    </div>
  );
}

export default App;
