import type { Skill } from "../types";
import { SkillCard } from "./SkillCard";

interface SkillListProps {
  skills: Skill[];
  emptyHint: string;
  onToggle: (skill: Skill) => void;
  onEdit: (skill: Skill) => void;
  onDelete: (skill: Skill) => void;
}

export function SkillList({ skills, emptyHint, onToggle, onEdit, onDelete }: SkillListProps) {
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
