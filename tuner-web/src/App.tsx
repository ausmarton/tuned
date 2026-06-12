import { useCallback, useEffect, useRef, useState } from "react";
import { MicEngine } from "./audio/MicEngine";
import type { FromWorker, ToWorker } from "./audio/protocol";
import type { ChordResult, Snapshot } from "./types";
import { ChordSmoother, StrumSmoother, type SmoothedString } from "./smoothing";
import { NavBar, type Mode } from "./components/NavBar";
import { TuningPicker } from "./components/TuningPicker";
import { TuneView } from "./components/TuneView";
import { StrumView } from "./components/StrumView";
import { ChordView } from "./components/ChordView";

export function App() {
  const workerRef = useRef<Worker | null>(null);
  const engineRef = useRef<MicEngine | null>(null);
  const pollRef = useRef<number | null>(null);
  const wakeRef = useRef<WakeLockSentinel | null>(null);
  const strumSmoother = useRef(new StrumSmoother());
  const chordSmoother = useRef(new ChordSmoother());
  const modeRef = useRef<Mode>("tune");

  const [running, setRunning] = useState(false);
  const [tuningId, setTuningId] = useState("guitar.standard");
  const [mode, setMode] = useState<Mode>("tune");
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [strum, setStrum] = useState<SmoothedString[]>([]);
  const [chord, setChord] = useState<ChordResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    modeRef.current = mode;
  }, [mode]);

  const post = useCallback((m: ToWorker) => workerRef.current?.postMessage(m), []);

  useEffect(() => {
    const worker = new Worker(new URL("./audio/tuner.worker.ts", import.meta.url), { type: "module" });
    worker.onmessage = (e: MessageEvent<FromWorker>) => {
      const msg = e.data;
      const now = performance.now();
      switch (msg.type) {
        case "snapshot":
          setSnapshot(msg.snapshot);
          break;
        case "strum":
          setStrum(strumSmoother.current.update(msg.report, now));
          break;
        case "chord":
          setChord(chordSmoother.current.update(msg.result, now));
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

  const acquireWake = useCallback(async () => {
    if (!("wakeLock" in navigator)) return;
    try {
      wakeRef.current = await navigator.wakeLock.request("screen");
    } catch {
      wakeRef.current = null;
    }
  }, []);

  const releaseWake = useCallback(async () => {
    await wakeRef.current?.release();
    wakeRef.current = null;
  }, []);

  const start = useCallback(async () => {
    if (running) return;
    try {
      const engine = new MicEngine((samples) => post({ type: "samples", samples }));
      const sampleRate = await engine.start();
      engineRef.current = engine;
      post({ type: "init", tuningId, sampleRate, a4: 440 });
      setRunning(true);
      setError(null);
      await acquireWake();
      pollRef.current = window.setInterval(() => {
        post({ type: "poll" });
        if (modeRef.current === "strum") post({ type: "strum" });
        else if (modeRef.current === "chords") post({ type: "chord" });
      }, 100);
    } catch {
      setError("Could not start the microphone.");
    }
  }, [running, tuningId, post, acquireWake]);

  const stop = useCallback(async () => {
    if (pollRef.current != null) {
      window.clearInterval(pollRef.current);
      pollRef.current = null;
    }
    await engineRef.current?.stop();
    engineRef.current = null;
    await releaseWake();
    setRunning(false);
  }, [releaseWake]);

  // Re-acquire the screen wake lock when the tab becomes visible again.
  useEffect(() => {
    const onVis = () => {
      if (running && document.visibilityState === "visible") void acquireWake();
    };
    document.addEventListener("visibilitychange", onVis);
    return () => document.removeEventListener("visibilitychange", onVis);
  }, [running, acquireWake]);

  const changeTuning = useCallback(
    (id: string) => {
      setTuningId(id);
      post({ type: "setTuning", id });
      strumSmoother.current.reset();
      chordSmoother.current.reset();
      setStrum([]);
      setChord(null);
    },
    [post],
  );

  const changeMode = useCallback((m: Mode) => {
    setMode(m);
    strumSmoother.current.reset();
    chordSmoother.current.reset();
    setStrum([]);
    setChord(null);
  }, []);

  return (
    <main style={{ maxWidth: 640, margin: "0 auto", padding: 24, minHeight: "100vh" }}>
      <h1 style={{ textAlign: "center" }}>OpenTuner</h1>
      <div style={{ textAlign: "center", marginBottom: 12 }}>
        <TuningPicker value={tuningId} onChange={changeTuning} />{" "}
        {running ? (
          <button onClick={() => void stop()}>Stop</button>
        ) : (
          <button onClick={() => void start()}>Start listening</button>
        )}
      </div>
      {error && <p style={{ color: "#c62828", textAlign: "center" }}>{error}</p>}

      <section style={{ minHeight: 280 }}>
        {mode === "tune" && <TuneView snapshot={snapshot} />}
        {mode === "strum" && <StrumView strings={strum} />}
        {mode === "chords" && <ChordView chord={chord} />}
      </section>

      <NavBar mode={mode} onChange={changeMode} />
    </main>
  );
}
