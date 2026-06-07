import type { Snapshot, StrumReport, ChordResult } from "../types";

/** Messages sent from the main thread to the tuner worker. */
export type ToWorker =
  | { type: "init"; tuningId: string; sampleRate: number; a4: number }
  | { type: "samples"; samples: Float32Array }
  | { type: "setTuning"; id: string }
  | { type: "poll" }
  | { type: "strum" }
  | { type: "chord" };

/** Messages sent from the tuner worker back to the main thread. */
export type FromWorker =
  | { type: "ready" }
  | { type: "snapshot"; snapshot: Snapshot }
  | { type: "strum"; report: StrumReport }
  | { type: "chord"; result: ChordResult };
