import init, { WasmTuner } from "../../pkg/tuner_core.js";
import type { ToWorker, FromWorker } from "./protocol";

// Typed view of the worker global that avoids DOM/webworker lib conflicts.
const ctx = self as unknown as {
  postMessage(msg: FromWorker): void;
  onmessage: ((e: MessageEvent<ToWorker>) => void) | null;
};

let tuner: WasmTuner | null = null;

ctx.onmessage = async (e: MessageEvent<ToWorker>) => {
  const msg = e.data;
  switch (msg.type) {
    case "init":
      await init();
      tuner = new WasmTuner(msg.tuningId, msg.sampleRate, msg.a4);
      ctx.postMessage({ type: "ready" });
      break;
    case "samples":
      tuner?.pushSamples(msg.samples);
      break;
    case "setTuning":
      tuner?.setTuning(msg.id);
      break;
    case "poll":
      if (tuner) {
        ctx.postMessage({
          type: "snapshot",
          snapshot: JSON.parse(tuner.snapshotJson()),
        });
      }
      break;
    case "strum":
      if (tuner) {
        ctx.postMessage({
          type: "strum",
          report: JSON.parse(tuner.analyseStrumJson()),
        });
      }
      break;
    case "chord":
      if (tuner) {
        ctx.postMessage({
          type: "chord",
          result: JSON.parse(tuner.recogniseChordJson()),
        });
      }
      break;
  }
};
