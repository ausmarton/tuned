import type { Voicing } from "../types";

interface Props {
  strings: string[];
  voicing: Voicing;
}

/** Compact numeric chord shape: a fret per string under its label (x = muted). */
export function VoicingLine({ strings, voicing }: Props) {
  return (
    <div style={{ display: "flex", justifyContent: "center", gap: 12, margin: "4px 0" }}>
      {voicing.map((f, i) => (
        <div key={i} style={{ display: "flex", flexDirection: "column", alignItems: "center" }}>
          <span style={{ fontSize: 11, color: "#666" }}>{strings[i] ?? ""}</span>
          <span style={{ fontFamily: "monospace", fontWeight: 700 }}>{f === null ? "x" : f}</span>
        </div>
      ))}
    </div>
  );
}
