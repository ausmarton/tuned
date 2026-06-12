import type { ChordResult } from "../types";
import { VoicingLine } from "./VoicingLine";

interface Props {
  chord: ChordResult | null;
}

export function ChordView({ chord }: Props) {
  if (!chord || !chord.best) {
    return <p>Play a chord — the name and fingerings appear here.</p>;
  }
  const best = chord.best;
  const strings = chord.strings ?? [];
  const alternates = chord.candidates
    .filter((c) => c.name !== best.name && c.score >= best.score - 0.08)
    .slice(0, 2);

  return (
    <div style={{ textAlign: "center" }}>
      <div style={{ fontSize: "3rem", fontWeight: 700 }}>{best.name}</div>
      <div>confidence {(best.score * 100).toFixed(0)}%</div>
      <div style={{ margin: "8px 0" }}>
        {(best.voicings ?? []).slice(0, 3).map((v, i) => (
          <VoicingLine key={i} strings={strings} voicing={v} />
        ))}
      </div>
      {alternates.length > 0 && (
        <div>
          <h4>Other matches</h4>
          {alternates.map((c) => (
            <div key={c.name} style={{ margin: "6px 0" }}>
              <div>
                {c.name} ({(c.score * 100).toFixed(0)}%)
              </div>
              {c.voicings && c.voicings[0] && <VoicingLine strings={strings} voicing={c.voicings[0]} />}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
