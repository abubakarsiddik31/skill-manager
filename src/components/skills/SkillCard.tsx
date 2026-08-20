import type { Skill, ToolEntry } from "../../types";

interface SkillCardProps {
  skill: Skill;
  toolEntries: ToolEntry[];
  onToggle: (skill: Skill) => void;
  onOpen: (skill: Skill) => void;
}

export function SkillCard({ skill, toolEntries, onToggle, onOpen }: SkillCardProps) {
  // Every tool whose read-set includes this skill's folder sees it.
  const seers = toolEntries.filter((t) => t.folders.some((f) => f.tool === skill.tool));

  return (
    <div className={`skill-card ${skill.enabled ? "" : "disabled"}`} onClick={() => onOpen(skill)}>
      <div
        className={`toggle ${skill.enabled ? "on" : ""}`}
        onClick={(e) => {
          e.stopPropagation();
          onToggle(skill);
        }}
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
        {seers.length > 1 && (
          <div className="seen-by">
            <span className="seen-by-label">seen by</span>
            {seers.slice(0, 3).map((t) => (
              <span
                key={t.id}
                className={`chip ${t.folders.find((f) => f.tool === skill.tool)?.role === "compat" ? "compat" : ""}`}
              >
                {t.label}
              </span>
            ))}
            {seers.length > 3 && <span className="chip more">+{seers.length - 3}</span>}
          </div>
        )}
      </div>
    </div>
  );
}
