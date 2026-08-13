interface TopbarProps {
  title: string;
  subtitle: string;
  query: string;
  onQueryChange: (query: string) => void;
  onForgetProject?: () => void;
}

export function Topbar({ title, subtitle, query, onQueryChange, onForgetProject }: TopbarProps) {
  return (
    <div className="topbar">
      <div>
        <h1>{title}</h1>
        <span className="subtitle">{subtitle}</span>
      </div>
      <input
        className="search"
        placeholder="search skills..."
        value={query}
        onChange={(e) => onQueryChange(e.target.value)}
      />
      {onForgetProject && (
        <button className="icon-btn danger" onClick={onForgetProject}>
          forget project
        </button>
      )}
    </div>
  );
}
