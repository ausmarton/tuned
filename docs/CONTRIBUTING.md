# Contributing

Thanks for helping improve OpenTuner. The project is GPL-3.0-or-later; by
contributing you agree your changes ship under that license.

## Before you push

Run the same checks CI runs.

### Rust core

```bash
cd tuner-core
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo doc --no-deps --all-features
cargo audit          # advisory check (install with: cargo install cargo-audit)
```

### Android

```bash
cd tuner-android
./gradlew ktlintCheck
./gradlew testDebugUnitTest
./gradlew assembleDebug      # needs Android SDK + NDK r26+
```

### Web

```bash
cd tuner-web
npm install
tsc --noEmit
npm run lint
npm test
npm run build
```

## DSP changes

DSP correctness is the heart of this project. If you touch a DSP module:

1. Update [docs/DSP.md](DSP.md) to match the new behaviour.
2. Update the **golden test expected values explicitly** — never loosen an
   assertion just to make a test pass. If a value genuinely changes, change it
   deliberately and say why in the commit message.
3. Re-run the canary tests (see [docs/TESTING.md](TESTING.md)).

## Toolchain / MSRV

The crate declares `rust-version = "1.75"`. It is developed and verified on
current stable (Rust 1.96+). On Rust 1.75 some transitive **dev**-dependencies
that bumped their MSRV to 1.80+ would need pinning; on a modern toolchain no
pins are necessary and `Cargo.toml` intentionally carries none. If you must
support 1.75, pin dev-deps locally rather than committing the pins.

## Style

- Rust: `cargo fmt`; keep clippy `pedantic`/`nursery` clean (the crate opts in).
- Kotlin: ktlint (`kotlin.code.style=official`).
- TypeScript: `strict` mode, ESLint with `--max-warnings 0`.
