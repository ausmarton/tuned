import { describe, it, expect } from "vitest";
import { StrumSmoother, ChordSmoother } from "../src/smoothing";
import type { ChordResult, StrumReport } from "../src/types";

function strum(cents: number | null, dir: "flat" | "in_tune" | "sharp" | null, conf: number): StrumReport {
  return {
    strings: [
      {
        index: 0,
        name: "E2",
        targetHz: 82,
        detectedHz: cents == null ? null : 82,
        centsOff: cents,
        direction: dir,
        confidence: conf,
      },
    ],
  };
}

function chord(name: string | null): ChordResult {
  return {
    candidates: name ? [{ name, score: 0.9 }] : [],
    best: name ? { name, score: 0.9 } : null,
    strings: [],
  };
}

describe("StrumSmoother", () => {
  it("holds the last confident reading then drops it", () => {
    const sm = new StrumSmoother(1000, 0.2);
    expect(sm.update(strum(3, "sharp", 0.9), 0)[0].centsOff).toBe(3);
    expect(sm.update(strum(null, null, 0), 500)[0].centsOff).toBe(3); // held
    expect(sm.update(strum(null, null, 0), 2000)[0].centsOff).toBeNull(); // expired
  });

  it("ignores low-confidence readings", () => {
    const sm = new StrumSmoother(1000, 0.5);
    expect(sm.update(strum(3, "sharp", 0.1), 0)[0].centsOff).toBeNull();
  });
});

describe("ChordSmoother", () => {
  it("debounces before showing", () => {
    const sm = new ChordSmoother(250, 1000);
    expect(sm.update(chord("C"), 0)).toBeNull();
    expect(sm.update(chord("C"), 100)).toBeNull();
    expect(sm.update(chord("C"), 300)).not.toBeNull();
  });

  it("holds across a brief gap then expires", () => {
    const sm = new ChordSmoother(0, 1000);
    expect(sm.update(chord("C"), 0)).toBeNull();
    expect(sm.update(chord("C"), 10)).not.toBeNull();
    expect(sm.update(chord(null), 500)).not.toBeNull();
    expect(sm.update(chord(null), 1100)).toBeNull();
  });
});
