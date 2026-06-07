import { SUPPORTED_TUNINGS } from "../types";

interface Props {
  value: string;
  onChange: (id: string) => void;
}

export function TuningPicker({ value, onChange }: Props) {
  return (
    <select
      aria-label="Tuning"
      value={value}
      onChange={(e) => onChange(e.target.value)}
    >
      {SUPPORTED_TUNINGS.map(([id, name]) => (
        <option key={id} value={id}>
          {name}
        </option>
      ))}
    </select>
  );
}
