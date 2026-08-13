import { useMemo, useState } from "react";
import { EditorModal } from "./components/EditorModal";
import { Sidebar } from "./components/Sidebar";
import { SkillList } from "./components/SkillList";
import { Topbar } from "./components/Topbar";
import { useGlobalSkills } from "./hooks/useGlobalSkills";
import { useProjects } from "./hooks/useProjects";
import { useProjectSkills } from "./hooks/useProjectSkills";
import { filterSkills } from "./lib/filterSkills";
import type { AgentTool, ProjectInfo, Skill, View } from "./types";
import "./App.css";

const ALL: AgentTool | "all" = "all";

function App() {
  const [view, setView] = useState<View>({ kind: "global" });
  const [activeTool, setActiveTool] = useState<AgentTool | "all">(ALL);
  const [query, setQuery] = useState("");
  const [editing, setEditing] = useState<Skill | null>(null);

  const global = useGlobalSkills();
  const projects = useProjects();
  const activeProject = view.kind === "project" ? view.project : null;
  const projectView = useProjectSkills(activeProject);

  function selectAll() {
    setView({ kind: "global" });
    setActiveTool(ALL);
  }

  function selectTool(tool: AgentTool) {
    setView({ kind: "global" });
    setActiveTool(tool);
  }

  function openProject(project: ProjectInfo) {
    setView({ kind: "project", project });
  }

  async function addProject() {
    const project = await projects.pickAndAdd();
    if (project) openProject(project);
  }

  async function forgetProject(project: ProjectInfo) {
    await projects.forget(project);
    if (activeProject?.path === project.path) setView({ kind: "global" });
  }

  const filteredGlobal = useMemo(
    () => filterSkills(global.skills, query, activeTool),
    [global.skills, query, activeTool],
  );

  const filteredProjectSkills = useMemo(
    () => filterSkills(projectView.skills, query),
    [projectView.skills, query],
  );

  const countForTool = (tool: AgentTool) => global.skills.filter((s) => s.tool === tool).length;

  const title =
    view.kind === "global"
      ? activeTool === ALL
        ? "all skills"
        : global.tools.find((t) => t.tool === activeTool)?.label ?? ""
      : view.project.name;

  const subtitle =
    view.kind === "global" ? `${filteredGlobal.length} shown` : view.project.path;

  return (
    <div className="app">
      <Sidebar
        tools={global.tools}
        totalSkillCount={global.skills.length}
        countForTool={countForTool}
        projects={projects.projects}
        view={view}
        activeTool={activeTool}
        onSelectAll={selectAll}
        onSelectTool={selectTool}
        onOpenProject={openProject}
        onAddProject={addProject}
      />

      <main className="main">
        <Topbar
          title={title}
          subtitle={subtitle}
          query={query}
          onQueryChange={setQuery}
          onForgetProject={view.kind === "project" ? () => forgetProject(view.project) : undefined}
        />

        <div className="skill-list">
          {view.kind === "global" ? (
            <SkillList
              skills={filteredGlobal}
              emptyHint="No skills found."
              onToggle={global.toggle}
              onEdit={setEditing}
              onDelete={global.remove}
            />
          ) : projectView.loading ? (
            <div className="empty-state">loading...</div>
          ) : (
            <SkillList
              skills={filteredProjectSkills}
              emptyHint="No skills found in this project."
              onToggle={projectView.toggle}
              onEdit={setEditing}
              onDelete={projectView.remove}
            />
          )}
        </div>
      </main>

      {editing && <EditorModal skill={editing} onClose={() => setEditing(null)} />}
    </div>
  );
}

export default App;
