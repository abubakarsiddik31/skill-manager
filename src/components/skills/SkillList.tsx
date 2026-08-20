import type { Skill, ToolEntry } from "../../types";
import { SkillCard } from "./SkillCard";

interface SkillListProps {
  skills: Skill[];
  toolEntries: ToolEntry[];
  emptyHint: string;
  onToggle: (skill: Skill) => void;
  onOpen: (skill: Skill) => void;
}

export function SkillList({ skills, toolEntries, emptyHint, onToggle, onOpen }: SkillListProps) {
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
        <SkillCard
          key={skill.id}
          skill={skill}
          toolEntries={toolEntries}
          onToggle={onToggle}
          onOpen={onOpen}
        />
      ))}
    </>
  );
}
