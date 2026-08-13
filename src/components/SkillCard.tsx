import type { Skill } from "../types";

interface SkillCardProps {
  skill: Skill;
  onToggle: (skill: Skill) => void;
  onEdit: (skill: Skill) => void;
  onDelete: (skill: Skill) => void;
}

export function SkillCard({ skill, onToggle, onEdit, onDelete }: SkillCardProps) {
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
