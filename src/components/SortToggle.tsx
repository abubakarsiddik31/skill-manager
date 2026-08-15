interface SortToggleProps {
  value: string;
  options: { id: string; label: string }[];
  onChange: (id: string) => void;
}

/** Small segmented control for picking a list's sort order. */
export function SortToggle({ value, options, onChange }: SortToggleProps) {
  return (
    <div className="sort-toggle">
      {options.map((o) => (
        <button
          key={o.id}
          className={value === o.id ? "active" : ""}
          onClick={() => onChange(o.id)}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}
