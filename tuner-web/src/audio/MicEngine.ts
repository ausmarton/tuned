/**
 * Wraps the Web Audio API: requests the microphone with all browser processing
 * disabled, runs a {@link AudioWorklet} that forwards Float32 sample chunks, and
 * hands them to a callback (which posts them to the tuner worker).
 */
export class MicEngine {
  private ctx: AudioContext | null = null;
  private stream: MediaStream | null = null;
  private node: AudioWorkletNode | null = null;

  constructor(private readonly onSamples: (samples: Float32Array) => void) {}

  /** Start capture; resolves with the actual sample rate of the audio context. */
  async start(): Promise<number> {
    this.stream = await navigator.mediaDevices.getUserMedia({
      audio: {
        echoCancellation: false,
        noiseSuppression: false,
        autoGainControl: false,
      },
    });
    this.ctx = new AudioContext();
    await this.ctx.audioWorklet.addModule(
      new URL("./worklet/capture-worklet.js", import.meta.url),
    );
    const source = this.ctx.createMediaStreamSource(this.stream);
    this.node = new AudioWorkletNode(this.ctx, "capture-worklet");
    this.node.port.onmessage = (e: MessageEvent<Float32Array>) => {
      this.onSamples(e.data);
    };
    source.connect(this.node);
    // Intentionally not connected to destination — we don't want playback.
    return this.ctx.sampleRate;
  }

  async stop(): Promise<void> {
    this.node?.disconnect();
    this.stream?.getTracks().forEach((t) => t.stop());
    await this.ctx?.close();
    this.ctx = null;
    this.node = null;
    this.stream = null;
  }
}
