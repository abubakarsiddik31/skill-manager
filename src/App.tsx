import { useEffect, useMemo, useRef, useState } from "react";
import { AddProjectModal } from "./components/modals/AddProjectModal";
import { BrowseView } from "./components/browse/BrowseView";
import { CreateSkillModal } from "./components/modals/CreateSkillModal";
import { EditorModal } from "./components/modals/EditorModal";
import { ProjectsModal } from "./components/modals/ProjectsModal";
import { Sidebar } from "./components/layout/Sidebar";
import { SkillList } from "./components/skills/SkillList";
import { Topbar } from "./components/layout/Topbar";
import { useGlobalSkills } from "./hooks/useGlobalSkills";
import { usePinnedTools } from "./hooks/usePinnedTools";
import { useProjects } from "./hooks/useProjects";
import { useProjectSkills } from "./hooks/useProjectSkills";
import { filterSkills } from "./utils/filterSkills";
import type { AgentTool, ProjectInfo, Skill, ToolEntry, View } from "./types";
import "./App.css";

const ALL = "all" as const;

/** Where the browse view should preinstall, captured when it opens. */
interface BrowseDefaults {
  tool?: AgentTool;
  project: ProjectInfo | null;
}

function App() {
  const [view, setView] = useState<View>({ kind: "global" });
  const [activeToolId, setActiveToolId] = useState<string | typeof ALL>(ALL);
  const [query, setQuery] = useState("");
  const [editing, setEditing] = useState<Skill | null>(null);
  const [addingProject, setAddingProject] = useState(false);
  const [creatingSkill, setCreatingSkill] = useState(false);
  const [browseDefaults, setBrowseDefaults] = useState<BrowseDefaults>({ project: null });
  const [browseFrom, setBrowseFrom] = useState<View>({ kind: "global" });
  const [showingAllProjects, setShowingAllProjects] = useState(false);
  const skillListRef = useRef<HTMLDivElement>(null);

  const global = useGlobalSkills();
  const projects = useProjects();
  const pinnedTools = usePinnedTools();
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
    projects.touch(project); // records the open for latest-first ordering
  }

  function openBrowse() {
    // Capture where we came from so "back" restores it, along with the
    // tool/project context the install picker should default to.
    setBrowseFrom(view);
    setBrowseDefaults({
      tool: activeTool
        ? (activeTool.folders.find((f) => f.role === "own")?.tool ??
            activeTool.folders[0]?.tool)
        : undefined,
      project: activeProject,
    });
    setView({ kind: "browse" });
  }

  function closeBrowse() {
    setView(browseFrom.kind === "browse" ? { kind: "global" } : browseFrom);
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

  async function removeProjectSkill(skill: Skill) {
    await projectView.remove(skill);
    await projects.refresh(); // keep sidebar skill-count badges honest
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

  // The browse view renders its own header, so these only describe the
  // global/project views the Topbar actually appears in.
  const title =
    view.kind === "project" ? view.project.name : activeTool ? activeTool.label : "all skills";

  const subtitle =
    view.kind === "project" ? view.project.path : `${filteredGlobal.length} shown`;

  return (
    <div className="app">
      <Sidebar
        toolEntries={global.toolEntries}
        totalSkillCount={global.skills.length}
        countForEntry={countForEntry}
        pinnedTools={pinnedTools.pinned}
        onTogglePinTool={pinnedTools.toggle}
        projects={projects.projects}
        skillCounts={projects.skillCounts}
        onTogglePinProject={projects.togglePin}
        onShowAllProjects={() => setShowingAllProjects(true)}
        view={view}
        activeToolId={activeToolId}
        onSelectAll={selectAll}
        onSelectTool={selectTool}
        onOpenProject={openProject}
        onAddProject={() => setAddingProject(true)}
        onBrowse={openBrowse}
      />

      <main className="main">
        {view.kind === "browse" ? (
          <BrowseView
            toolEntries={global.toolEntries}
            projects={projects.projects}
            defaultTool={browseDefaults.tool}
            defaultProject={browseDefaults.project}
            onBack={closeBrowse}
            onInstalled={async (skill) => {
              await global.refresh();
              if (skill.scope === "project") {
                await projects.refresh();
                projectView.reload();
              }
            }}
          />
        ) : (
          <>
            <Topbar
              title={title}
              subtitle={subtitle}
              folders={view.kind === "global" ? activeTool?.folders : undefined}
              query={query}
              onQueryChange={setQuery}
              onForgetProject={
                view.kind === "project" ? () => forgetProject(view.project) : undefined
              }
              onBrowse={openBrowse}
              onNewSkill={() => setCreatingSkill(true)}
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
          </>
        )}
      </main>

      {editing && (
        <EditorModal
          skill={editing}
          toolEntries={global.toolEntries}
          onClose={() => setEditing(null)}
          onDelete={view.kind === "global" ? global.remove : removeProjectSkill}
        />
      )}

      {creatingSkill && (
        <CreateSkillModal
          toolEntries={global.toolEntries}
          projects={projects.projects}
          activeProject={activeProject}
          defaultTool={
            activeTool
              ? (activeTool.folders.find((f) => f.role === "own")?.tool ??
                activeTool.folders[0]?.tool) as AgentTool | undefined
              : undefined
          }
          onClose={() => setCreatingSkill(false)}
          onCreated={async (skill) => {
            setCreatingSkill(false);
            await global.refresh();
            if (skill.scope === "project") {
              await projects.refresh(); // keep sidebar skill-count badges honest
              projectView.reload();
            }
            setEditing(skill); // the instructions are the user's to write
          }}
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

      {showingAllProjects && (
        <ProjectsModal
          projects={projects.projects}
          skillCounts={projects.skillCounts}
          activePath={activeProject?.path}
          onClose={() => setShowingAllProjects(false)}
          onOpen={openProject}
        />
      )}
    </div>
  );
}

export default App;
