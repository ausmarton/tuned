import type { ChordResult, Direction, StrumReport } from "./types";

/**
 * A per-string strum reading after temporal smoothing. `ageMs` is how long ago
 * the (held) reading was last confidently observed; the UI fades the indicator
 * as it ages.
 */
export interface SmoothedString {
  name: string;
  centsOff: number | null;
  direction: Direction | null;
  ageMs: number;
  confidence: number;
}

interface LastReading {
  cents: number;
  direction: Direction;
  confidence: number;
  atMs: number;
}

/**
 * Holds each string's last confident reading for up to `holdMs` so the live
 * strum display stays legible between pluck transients. A reading counts only
 * when its confidence clears `minConfidence`.
 */
export class StrumSmoother {
  private last = new Map<number, LastReading>();

  constructor(
    private readonly holdMs = 2000,
    private readonly minConfidence = 0.2,
  ) {}

  update(report: StrumReport, nowMs: number): SmoothedString[] {
    return report.strings.map((s, i) => {
      if (s.centsOff != null && s.direction != null && s.confidence >= this.minConfidence) {
        this.last.set(i, {
          cents: s.centsOff,
          direction: s.direction,
          confidence: s.confidence,
          atMs: nowMs,
        });
      }
      const l = this.last.get(i);
      if (l && nowMs - l.atMs <= this.holdMs) {
        return {
          name: s.name,
          centsOff: l.cents,
          direction: l.direction,
          ageMs: nowMs - l.atMs,
          confidence: l.confidence,
        };
      }
      return { name: s.name, centsOff: null, direction: null, ageMs: Number.MAX_SAFE_INTEGER, confidence: 0 };
    });
  }

  reset(): void {
    this.last.clear();
  }
}

/**
 * Debounces the live chord display: a new best chord is only shown once it has
 * persisted for `debounceMs`, and the last shown chord is held for up to
 * `holdMs` across brief silences, so the readout doesn't flicker.
 */
export class ChordSmoother {
  private pendingName: string | null = null;
  private pendingSince = 0;
  private displayed: ChordResult | null = null;
  private displayedAt = 0;

  constructor(
    private readonly debounceMs = 250,
    private readonly holdMs = 1500,
  ) {}

  update(result: ChordResult, nowMs: number): ChordResult | null {
    const best = result.best;
    if (best) {
      if (best.name === this.pendingName) {
        if (nowMs - this.pendingSince >= this.debounceMs) {
          this.displayed = result;
          this.displayedAt = nowMs;
        }
      } else {
        this.pendingName = best.name;
        this.pendingSince = nowMs;
      }
    } else {
      this.pendingName = null;
    }
    return this.displayed && nowMs - this.displayedAt <= this.holdMs ? this.displayed : null;
  }

  reset(): void {
    this.pendingName = null;
    this.displayed = null;
  }
}
