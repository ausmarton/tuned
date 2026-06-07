# Original requirements gathering

Captured from the very first turn of the OpenTuner conversation, so the
intent isn't lost in retelling.

## User's opening message

> Let's build a simple instrument tuner app that can be open source and
> free. We want the app to focus on tuning guitars (six string), basses,
> and a guitarra portuguesa in Lisboa and Coimbra tuning.
>
> While we can do using string by string tuning and auto-tuning by
> identifying which notes are played, and indicate how it should be tuned,
> we additionally need the following features too.
>
> 1. we can strum the instrument and all strings are analysed and we can
>    provide which ones need to be fine tuned and which way as well.
> 2. We want to be able to strum a chord or play a chord and the app
>    should identify the chord.
>
> Write all tests and good engineering practices to ensure we have the
> best example of the best engineered apps.

## Q&A round 1

**Q: What platform should this target?**
A: "Let's build this in a way that we can focus on Android. Browser based
would be nice to have if it can be achieved for free, without much extra
effort. iOS is a thing we might want to think about later — as of now, we
don't have any plans for iOS given it needs Apple hardware to publish the
app. We only have android devices and browsers to run this on for the
moment. Does this make sense?"

**Q: Which tech stack do you prefer?**
A: "Whichever stack allows us to build the best possible tech around tuning,
sound analyses, filtering noise and other stuff. Think about what challenges
we might face when trying to deliver the features we want and then pick the
one which has the best support for those features to be implemented. Rust
based would be preferable, but think about any audio issues we might face
when trying to build this full feature set. We really need to work in noisy
environments and be able to identify each string and also be able to
identify a full strum etc."

**Q: How polished should this initial build be?**
A: "The MVP with core features, should be covered comprehensively with
tests and CI, docs, etc. But we also want to get the DSP rigorously tested
and correct."

## Q&A round 2

**Q: Which open-source license?**
A: GPL-3.0 (strong copyleft — derivatives stay open)

**Q: Should the tuning database be extensible to other instruments?**
A: "We can focus on other instruments in future builds, but for now, for
the initial build/MVP, let's focus on the instruments we've agreed upon"

**Q: Alternate tunings beyond standard?**
A: Just guitarra portuguesa Lisboa + Coimbra

## Derived constraints (from the answers above)

- **No iOS work**, even speculatively. Don't add iOS-flavoured files,
  CocoaPods config, Swift bindings, or anything else that suggests it's
  planned.
- **Rust is preferred but the deciding factor is DSP capability for noisy
  environments**. We picked Rust + Android + WASM precisely because the
  same Rust core serves both targets and the DSP ecosystem there is solid
  (rustfft, num-complex, libm).
- **Comprehensive tests and CI are part of MVP scope** — they're not
  follow-up work. The previous session's 110 tests + clippy + benches
  reflect this.
- **DSP correctness is rigorously prioritised** — this is why the previous
  session spent so much time finding and fixing the chroma bug, the strum
  cross-talk, and the buffer-length problem.
- **Scope is strict**: 4 tunings (guitar standard, bass standard, guitarra
  Lisboa, guitarra Coimbra). No alt guitar tunings. No 7-string, no
  ukulele, no mandolin, no banjo. The tuning DB is designed to be
  extensible (`pub const ALL: &[Tuning] = ...`), but adding entries is a
  follow-up release.
