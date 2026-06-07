import type { Snapshot, StrumReport, ChordResult } from "../types";
import { TuningPicker } from "./TuningPicker";
import { PitchDisplay } from "./PitchDisplay";
import { StrumGrid } from "./StrumGrid";
import { ChordDisplay } from "./ChordDisplay";

interface Props {
  running: boolean;
  tuningId: string;
  snapshot: Snapshot | null;
  strum: StrumReport | null;
  chord: ChordResult | null;
  onStart: () => void;
  onStop: () => void;
  onTuningChange: (id: string) => void;
  onAnalyseStrum: () => void;
  onRecogniseChord: () => void;
}

export function TunerScreen(props: Props) {
  return (
    <main style={{ maxWidth: 640, margin: "0 auto", padding: 24, textAlign: "center" }}>
      <h1>OpenTuner</h1>

      <TuningPicker value={props.tuningId} onChange={props.onTuningChange} />

      <PitchDisplay snapshot={props.snapshot} />

      <div style={{ display: "flex", gap: 8, justifyContent: "center", margin: "16px 0" }}>
        <button onClick={props.running ? props.onStop : props.onStart}>
          {props.running ? "Stop" : "Start"}
        </button>
        <button onClick={props.onAnalyseStrum} disabled={!props.running}>
          Analyse strum
        </button>
        <button onClick={props.onRecogniseChord} disabled={!props.running}>
          Identify chord
        </button>
      </div>

      <StrumGrid report={props.strum} />
      <ChordDisplay result={props.chord} />
    </main>
  );
}
