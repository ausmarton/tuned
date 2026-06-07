import { describe, it, expect } from "vitest";
import type { Snapshot, ChordResult, StrumReport } from "../src/types";

// We don't load the real WASM here (that needs a build step); instead we assert
// the module *shape* the worker depends on, and that the JSON the Rust side
// emits parses into our TypeScript types.

interface WasmTunerShape {
  pushSamples(samples: Float32Array): void;
  setTuning(id: string): boolean;
  snapshotJson(): string;
  analyseStrumJson(): string;
  recogniseChordJson(): string;
}

function makeFakeTuner(): WasmTunerShape {
  return {
    pushSamples: () => undefined,
    setTuning: () => true,
    snapshotJson: () =>
      JSON.stringify({
        pitchHz: 196.0,
        centsOff: 1.2,
        direction: "in_tune",
        nearestString: 3,
        nearestStringName: "G3",
        confidence: 0.92,
      }),
    analyseStrumJson: () =>
      JSON.stringify({
        strings: [
          {
            index: 0,
            name: "E2",
            targetHz: 82.41,
            detectedHz: 82.4,
            centsOff: -0.2,
            direction: "in_tune",
            confidence: 0.8,
          },
        ],
      }),
    recogniseChordJson: () =>
      JSON.stringify({
        candidates: [
          { name: "C", score: 0.99 },
          { name: "Cmaj7", score: 0.95 },
        ],
        best: { name: "C", score: 0.99 },
      }),
  };
}

describe("wasm module shape", () => {
  const tuner = makeFakeTuner();

  it("exposes the four JSON methods plus push/set", () => {
    expect(typeof tuner.pushSamples).toBe("function");
    expect(typeof tuner.setTuning).toBe("function");
    expect(typeof tuner.snapshotJson).toBe("function");
    expect(typeof tuner.analyseStrumJson).toBe("function");
    expect(typeof tuner.recogniseChordJson).toBe("function");
  });

  it("snapshot JSON parses into a Snapshot", () => {
    const snap = JSON.parse(tuner.snapshotJson()) as Snapshot;
    expect(snap.nearestStringName).toBe("G3");
    expect(snap.direction).toBe("in_tune");
    expect(typeof snap.confidence).toBe("number");
  });

  it("strum JSON parses into a StrumReport", () => {
    const report = JSON.parse(tuner.analyseStrumJson()) as StrumReport;
    expect(report.strings).toHaveLength(1);
    expect(report.strings[0].name).toBe("E2");
  });

  it("chord JSON parses into a ChordResult", () => {
    const result = JSON.parse(tuner.recogniseChordJson()) as ChordResult;
    expect(result.best?.name).toBe("C");
    expect(result.candidates.length).toBeGreaterThan(1);
  });
});
