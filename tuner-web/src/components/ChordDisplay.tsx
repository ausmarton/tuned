import type { ChordResult } from "../types";

interface Props {
  result: ChordResult | null;
}

export function ChordDisplay({ result }: Props) {
  if (!result) return null;
  const best = result.best;
  return (
    <div className="chord-display">
      <div style={{ fontSize: "2rem", fontWeight: 700 }}>
        {best ? best.name : "—"}
      </div>
      {best && <div>confidence {(best.score * 100).toFixed(0)}%</div>}
      <ol>
        {result.candidates.slice(0, 3).map((c) => (
          <li key={c.name}>
            {c.name} ({(c.score * 100).toFixed(0)}%)
          </li>
        ))}
      </ol>
    </div>
  );
}
