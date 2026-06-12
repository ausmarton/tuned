import type { SmoothedString } from "../smoothing";
import { CentsMeter, meterColor } from "./CentsMeter";

interface Props {
  strings: SmoothedString[];
}

export function StrumView({ strings }: Props) {
  if (strings.length === 0) {
    return <p>Strum your instrument — each string updates live as it rings.</p>;
  }
  return (
    <div>
      <p>Strum and tune — readings hold for a moment so you can adjust.</p>
      {strings.map((s) => {
        const fade = s.centsOff === null ? 0.35 : Math.max(0.4, Math.min(1, 1 - s.ageMs / 2000));
        return (
          <div
            key={s.name}
            style={{ display: "flex", alignItems: "center", gap: 12, opacity: fade, margin: "6px 0" }}
          >
            <span style={{ width: 44, fontWeight: 700 }}>{s.name}</span>
            <span style={{ width: 64, color: meterColor(s.centsOff) }}>
              {s.centsOff != null ? `${s.centsOff >= 0 ? "+" : ""}${s.centsOff.toFixed(1)}¢` : "—"}
            </span>
            <div style={{ flex: 1 }}>
              <CentsMeter cents={s.centsOff} height={28} />
            </div>
          </div>
        );
      })}
    </div>
  );
}
