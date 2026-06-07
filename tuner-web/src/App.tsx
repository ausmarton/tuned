import { useCallback, useEffect, useRef, useState } from "react";
import { TunerScreen } from "./components/TunerScreen";
import { MicEngine } from "./audio/MicEngine";
import type { FromWorker, ToWorker } from "./audio/protocol";
import type { Snapshot, StrumReport, ChordResult } from "./types";

export function App() {
  const workerRef = useRef<Worker | null>(null);
  const engineRef = useRef<MicEngine | null>(null);
  const pollRef = useRef<number | null>(null);

  const [running, setRunning] = useState(false);
  const [tuningId, setTuningId] = useState("guitar.standard");
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [strum, setStrum] = useState<StrumReport | null>(null);
  const [chord, setChord] = useState<ChordResult | null>(null);

  const post = useCallback((msg: ToWorker) => {
    workerRef.current?.postMessage(msg);
  }, []);

  // Create the worker once.
  useEffect(() => {
    const worker = new Worker(
      new URL("./audio/tuner.worker.ts", import.meta.url),
      { type: "module" },
    );
    worker.onmessage = (e: MessageEvent<FromWorker>) => {
      const msg = e.data;
      switch (msg.type) {
        case "snapshot":
          setSnapshot(msg.snapshot);
          break;
        case "strum":
          setStrum(msg.report);
          break;
        case "chord":
          setChord(msg.result);
          break;
        case "ready":
          break;
      }
    };
    workerRef.current = worker;
    return () => {
      worker.terminate();
      workerRef.current = null;
    };
  }, []);

  const start = useCallback(async () => {
    if (running) return;
    const engine = new MicEngine((samples) => {
      post({ type: "samples", samples });
    });
    const sampleRate = await engine.start();
    engineRef.current = engine;
    post({ type: "init", tuningId, sampleRate, a4: 440 });
    setRunning(true);
    pollRef.current = window.setInterval(() => post({ type: "poll" }), 100);
  }, [running, tuningId, post]);

  const stop = useCallback(async () => {
    if (pollRef.current != null) {
      window.clearInterval(pollRef.current);
      pollRef.current = null;
    }
    await engineRef.current?.stop();
    engineRef.current = null;
    setRunning(false);
  }, []);

  const changeTuning = useCallback(
    (id: string) => {
      setTuningId(id);
      post({ type: "setTuning", id });
    },
    [post],
  );

  return (
    <TunerScreen
      running={running}
      tuningId={tuningId}
      snapshot={snapshot}
      strum={strum}
      chord={chord}
      onStart={() => void start()}
      onStop={() => void stop()}
      onTuningChange={changeTuning}
      onAnalyseStrum={() => post({ type: "strum" })}
      onRecogniseChord={() => post({ type: "chord" })}
    />
  );
}
