import type { Skill } from "../types";

interface SkillCardProps {
  skill: Skill;
  onToggle: (skill: Skill) => void;
  onOpen: (skill: Skill) => void;
}

export function SkillCard({ skill, onToggle, onOpen }: SkillCardProps) {
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
      </div>
    </div>
  );
}
