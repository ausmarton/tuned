export type Mode = "tune" | "strum" | "chords";

interface Props {
  mode: Mode;
  onChange: (m: Mode) => void;
}

const TABS: ReadonlyArray<readonly [Mode, string]> = [
  ["tune", "Tune"],
  ["strum", "Strum"],
  ["chords", "Chords"],
];

export function NavBar({ mode, onChange }: Props) {
  return (
    <nav
      style={{
        display: "flex",
        justifyContent: "center",
        gap: 8,
        position: "sticky",
        bottom: 0,
        padding: "12px 0",
        background: "var(--bg, #fff)",
        borderTop: "1px solid #ddd",
      }}
    >
      {TABS.map(([m, label]) => (
        <button
          key={m}
          onClick={() => onChange(m)}
          aria-pressed={m === mode}
          style={{
            padding: "8px 20px",
            fontWeight: m === mode ? 700 : 400,
            background: m === mode ? "#e8f0fe" : "transparent",
            border: "1px solid #ccc",
            borderRadius: 8,
            cursor: "pointer",
          }}
        >
          {label}
        </button>
      ))}
    </nav>
  );
}
