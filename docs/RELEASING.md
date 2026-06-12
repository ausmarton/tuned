# Releasing OpenTuner

Downloadable APKs are published as **GitHub Releases**, built and signed
automatically when you push a version tag (`v*`). This document covers the
one-time signing setup, the release flow, and the path to the Play Store.

## 1. One-time: create a signing keystore

Android requires every release to be signed, and **all updates to an app must
use the same key**. Create one keystore and keep it safe (losing it means you
can't ship updates under the same app identity).

```bash
keytool -genkeypair -v \
  -keystore opentuner-release.keystore \
  -alias opentuner -keyalg RSA -keysize 2048 -validity 10000 \
  -storepass <STORE_PASSWORD> -keypass <KEY_PASSWORD> \
  -dname "CN=OpenTuner, O=OpenTuner"
```

Keep this file **out of git** (it is gitignored, along with `keystore.properties`).

## 2. One-time: add GitHub Actions secrets

Base64-encode the keystore and add four repository secrets
(Settings → Secrets and variables → Actions):

```bash
base64 -w0 opentuner-release.keystore   # copy the output into KEYSTORE_BASE64
```

| Secret | Value |
|---|---|
| `KEYSTORE_BASE64` | base64 of the keystore file |
| `KEYSTORE_PASSWORD` | the store password |
| `KEY_ALIAS` | `opentuner` (or your alias) |
| `KEY_PASSWORD` | the key password |

`.github/workflows/release.yml` decodes the keystore and passes the passwords as
env vars; `tuner-android/app/build.gradle.kts` reads them for the `release`
signing config.

## 3. Building locally (optional)

To produce a signed APK on your machine, create a gitignored
`tuner-android/keystore.properties`:

```properties
storeFile=opentuner-release.keystore
storePassword=<STORE_PASSWORD>
keyAlias=opentuner
keyPassword=<KEY_PASSWORD>
```

Then `./gradlew assembleRelease` writes
`app/build/outputs/apk/release/app-release.apk`. Without a keystore configured,
release builds fall back to debug signing so the build still works.

## 4. Cutting a release

```bash
# bump versionCode / versionName in app/build.gradle.kts, commit, then:
git tag v0.2.0
git push origin v0.2.0
```

The `release` workflow builds the signed APK, attaches it (plus a `.sha256`
checksum) to a new GitHub Release, and generates release notes. Users download
`opentuner-vX.Y.Z.apk` and sideload it (Settings → allow install from this
source).

## 5. Later: Google Play Store

Publishing to Play is a separate effort, not covered by this pipeline:

- **Play App Signing** — Google holds the app signing key; you upload with an
  *upload key* (the keystore above can serve as the upload key).
- **Developer account** — one-time registration fee.
- **App bundle** — Play prefers `./gradlew bundleRelease` (`.aab`) over APK.
- **Store listing** — title, descriptions, screenshots, feature graphic.
- **Privacy policy** — required; OpenTuner records no data and needs only the
  microphone, which simplifies the data-safety form.
- **Target SDK** — Play enforces a recent `targetSdk`; keep it current.

When we tackle this, add a `bundleRelease` step and the Play Publisher action
to a separate workflow.
