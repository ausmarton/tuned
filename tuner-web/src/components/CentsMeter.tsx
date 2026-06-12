interface Props {
  cents: number | null;
  height?: number;
}

export function meterColor(cents: number | null): string {
  if (cents === null) return "#9e9e9e";
  return Math.abs(cents) <= 5 ? "#2e7d32" : "#e65100";
}

/** Horizontal ±50-cent meter with a moving needle (SVG). */
export function CentsMeter({ cents, height = 48 }: Props) {
  const w = 300;
  const midY = height / 2;
  const usable = w * 0.9;
  const centerX = w / 2;
  const needleX = cents === null ? null : centerX + (Math.max(-50, Math.min(50, cents)) / 50) * (usable / 2);

  return (
    <svg viewBox={`0 0 ${w} ${height}`} width="100%" height={height} role="img" aria-label="cents meter">
      <line x1={(w - usable) / 2} y1={midY} x2={(w + usable) / 2} y2={midY} stroke="#ccc" strokeWidth={2} />
      {[-50, -25, 0, 25, 50].map((c) => {
        const x = centerX + (c / 50) * (usable / 2);
        const th = c === 0 ? height * 0.4 : height * 0.22;
        return (
          <line
            key={c}
            x1={x}
            y1={midY - th}
            x2={x}
            y2={midY + th}
            stroke={c === 0 ? "#2e7d32" : "#9e9e9e"}
            strokeWidth={c === 0 ? 3 : 2}
          />
        );
      })}
      {needleX !== null && (
        <line
          x1={needleX}
          y1={midY - height * 0.45}
          x2={needleX}
          y2={midY + height * 0.45}
          stroke={meterColor(cents)}
          strokeWidth={4}
        />
      )}
    </svg>
  );
}
