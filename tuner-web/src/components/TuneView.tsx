import type { Snapshot } from "../types";
import { CentsMeter, meterColor } from "./CentsMeter";

interface Props {
  snapshot: Snapshot | null;
}

function directionLabel(cents: number): string {
  if (Math.abs(cents) <= 5) return "in tune";
  return cents < 0 ? "♭ flat" : "♯ sharp";
}

export function TuneView({ snapshot }: Props) {
  const hasPitch = snapshot != null && snapshot.pitchHz != null;
  const name = hasPitch ? (snapshot.nearestStringName ?? "—") : "—";
  const cents = hasPitch ? snapshot.centsOff : null;
  const color = meterColor(cents);

  return (
    <div style={{ textAlign: "center" }}>
      <div style={{ fontSize: "4rem", fontWeight: 700, color }}>{name}</div>
      {hasPitch && <div>{snapshot.pitchHz!.toFixed(1)} Hz</div>}
      <div style={{ color }}>
        {cents != null ? `${cents >= 0 ? "+" : ""}${cents.toFixed(1)} cents · ${directionLabel(cents)}` : "play a note"}
      </div>
      <div style={{ margin: "12px 0" }}>
        <CentsMeter cents={cents} height={64} />
      </div>
      <div>confidence {((snapshot?.confidence ?? 0) * 100).toFixed(0)}%</div>
    </div>
  );
}
