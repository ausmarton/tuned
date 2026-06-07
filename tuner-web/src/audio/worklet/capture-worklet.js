// AudioWorkletProcessor that copies its input buffer and posts it to the main
// thread. It runs in its own realm, so it must be served as a separate file.
class CaptureWorklet extends AudioWorkletProcessor {
  process(inputs) {
    const input = inputs[0];
    if (input && input[0]) {
      // Copy channel 0 (slice detaches it from the reused render buffer).
      this.port.postMessage(input[0].slice(0));
    }
    return true;
  }
}

registerProcessor("capture-worklet", CaptureWorklet);
