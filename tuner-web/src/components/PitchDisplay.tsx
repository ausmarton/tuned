import type { Snapshot } from "../types";

interface Props {
  snapshot: Snapshot | null;
}

function directionLabel(cents: number): string {
  if (Math.abs(cents) <= 5) return "in tune";
  return cents < 0 ? "♭ flat" : "♯ sharp";
}

export function PitchDisplay({ snapshot }: Props) {
  const hasPitch = snapshot != null && snapshot.pitchHz != null;
  const name = hasPitch ? snapshot.nearestStringName ?? "—" : "—";

  return (
    <div className="pitch-display">
      <div className="pitch-note" style={{ fontSize: "4rem", fontWeight: 700 }}>
        {name}
      </div>
      {hasPitch && (
        <>
          <div>{snapshot.pitchHz!.toFixed(1)} Hz</div>
          <div>
            {snapshot.centsOff != null
              ? `${snapshot.centsOff >= 0 ? "+" : ""}${snapshot.centsOff.toFixed(1)} cents · ${directionLabel(
                  snapshot.centsOff,
                )}`
              : ""}
          </div>
        </>
      )}
      <div>confidence {((snapshot?.confidence ?? 0) * 100).toFixed(0)}%</div>
    </div>
  );
}
