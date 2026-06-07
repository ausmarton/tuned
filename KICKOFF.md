# Claude Code Kick-off Prompt for OpenTuner

Paste the following into a fresh Claude Code session. It frames the work and
points Claude Code at `HANDOFF.md`, which is the full spec.

---

```
I'm continuing a project called OpenTuner — an open-source instrument tuner
for guitar, bass, and guitarra portuguesa (Lisboa and Coimbra tunings),
GPL-3.0 licensed, targeting Android primarily with a web app as a bonus.

The full technical spec, architectural decisions, list of known DSP bugs and
fixes, and file-by-file regeneration guide is in HANDOFF.md (in this same
directory). Read it first — it's long but heavily structured. Sections 1-3
are requirements and decisions, 4-5 are layout and toolchain, 6-7 capture
real bugs found and the verified test results, 8 is what's left to do, and
section 10 is the file-by-file spec.

Please:

1. Read HANDOFF.md end to end.
2. Confirm the toolchain — run `rustc --version`. If it's 1.80 or later,
   skip the dev-dependency pinning that section 5 describes.
3. Build the project per the order in section 11. After each Rust module,
   run `cargo test --lib` and stop if anything fails — the previous session
   verified 110 tests passing, so any failure is a regression from the spec.
4. Commit after every milestone (cents → tunings → pitch → fft+chroma →
   chord → strum → tuner → bindings → integration tests → benches).
5. When the Rust core is green (`cargo test`, `cargo clippy --all-targets
   -- -D warnings`), move on to the Android shell, then the web app, then
   CI workflows.

Two things I care about specifically:

- The chroma module MUST use triangular interpolation between adjacent
  pitch classes, not nearest-semitone rounding. The previous session found
  and fixed a real bug here. Section 6.1 and 10.11 describe it.
- The strum analyser MUST use a 3-cascaded-biquad (6th order) bandpass AND
  skip the first half of the filtered buffer before YIN. Section 6.2 and
  10.13 explain why.

If you find a discrepancy between HANDOFF.md and what the tests want, the
tests are authoritative — those were verified passing. Be candid when
something doesn't reproduce; don't fudge the assertions to make a test pass.

Let me know when the core is green and you're ready to move to the Android
shell.
```

---

## What to do BEFORE starting Claude Code

1. Create an empty directory for the project (e.g. `mkdir opentuner && cd
   opentuner`).
2. Drop `HANDOFF.md` into that directory.
3. Start Claude Code in that directory (`claude` from the terminal).
4. Paste the prompt above.

## Expected first-pass timing

On a normal developer machine with Rust stable already installed and the
crates cache populated, regenerating and verifying the Rust core from the
spec should take Claude Code roughly 30-60 minutes of agent time. The
Android and web shells add another 30-60 minutes each. The actual CI runs
on GitHub will only work once you've pushed to a repo.

## When to push back

If Claude Code wants to "improve" the chroma module by removing the
triangular interpolation, or wants to use a different chord template
weighting, or wants to drop the warmup-skip in the strum analyser —
**push back and point at the verified test results in HANDOFF.md section 7**.
Those design choices fix real bugs that pure sines don't show but real
instruments do.
