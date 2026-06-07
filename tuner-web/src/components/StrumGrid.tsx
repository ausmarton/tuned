import type { StrumReport } from "../types";

interface Props {
  report: StrumReport | null;
}

export function StrumGrid({ report }: Props) {
  if (!report) return null;
  return (
    <table className="strum-grid">
      <thead>
        <tr>
          <th>String</th>
          <th>Detected</th>
          <th>Cents</th>
          <th>Status</th>
        </tr>
      </thead>
      <tbody>
        {report.strings.map((s) => {
          const status =
            s.direction === null
              ? "—"
              : s.direction === "in_tune"
                ? "in tune"
                : s.direction === "flat"
                  ? "♭ flat"
                  : "♯ sharp";
          return (
            <tr key={s.index}>
              <td>{s.name}</td>
              <td>{s.detectedHz != null ? `${s.detectedHz.toFixed(1)} Hz` : "—"}</td>
              <td>
                {s.centsOff != null
                  ? `${s.centsOff >= 0 ? "+" : ""}${s.centsOff.toFixed(1)}`
                  : "—"}
              </td>
              <td>{status}</td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}
