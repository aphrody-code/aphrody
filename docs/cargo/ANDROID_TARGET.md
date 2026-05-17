<!-- SPDX-License-Identifier: Apache-2.0 -->
# Android target — Cross-compile via `cargo-ndk`

> Réf. : `rust-toolchain.toml`, `.cargo/config.toml`, `.github/workflows/cross-platform.yml`.
> Aligné [Android NDK guide](https://developer.android.com/ndk/guides/other_build_systems).

## Pré-requis

```bash
# 1. Android NDK (via Android Studio ou WinGet `Google.AndroidStudio`)
#    Path attendu sur Windows : C:\Users\<user>\AppData\Local\Android\Sdk\ndk\<version>
export ANDROID_NDK_HOME=$LOCALAPPDATA/Android/Sdk/ndk/26.3.11579264

# 2. cargo-ndk (wrapper qui automatise les linkers NDK)
cargo install --locked cargo-ndk

# 3. Targets Rust déjà déclarés dans `rust-toolchain.toml` :
#    - aarch64-linux-android  (ARM 64-bit, devices modernes)
#    - armv7-linux-androideabi (ARM 32-bit, legacy)
#    - x86_64-linux-android   (Android x64 emulator)
#    - i686-linux-android     (Android x86 emulator)
```

## Build commands

```bash
# Tous les targets ARM en un seul cmd
cargo ndk --platform 28 \
          -t armeabi-v7a -t arm64-v8a -t x86_64 \
          -o ./out/android/jniLibs \
          build --release -p aphrody

# Un seul target via alias maison
cargo ndk --platform 28 -t aarch64-linux-android build -p aphrody --locked --profile dist
```

Le flag `--platform 28` cible Android 9+ (API 28). Adapter selon support minimum.

## Configuration `.cargo/config.toml`

```toml
[target.aarch64-linux-android]
# Automatiquement injecté par cargo-ndk, mais documenté ici pour audit :
# linker = "$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/<host>/bin/aarch64-linux-android28-clang"

[target.armv7-linux-androideabi]
# linker = ".../armv7a-linux-androideabi28-clang"

[target.x86_64-linux-android]
# linker = ".../x86_64-linux-android28-clang"
```

## Intégration JNI (si besoin d'exposer le `cli` à du code Java/Kotlin)

```toml
# crates/cli/Cargo.toml — pour produire une lib JNI au lieu d'un exécutable
[lib]
crate-type = ["cdylib"]
name       = "aphrody"   # → libgoogle_cli.so
```

Côté Java/Kotlin :
```kotlin
class Aphrody {
    companion object { init { System.loadLibrary("aphrody") } }
    external fun version(): String
}
```

## CI Android

Voir `.github/workflows/cross-platform.yml` job `android` :

```yaml
android:
  needs: lint
  runs-on: ubuntu-latest
  strategy:
    matrix:
      target: [aarch64-linux-android, x86_64-linux-android]
  steps:
    - uses: nttld/setup-ndk@v1
      with: { ndk-version: r26d }
    - uses: taiki-e/install-action@v2
      with: { tool: cargo-ndk }
    - run: cargo ndk --platform 28 --target ${{ matrix.target }} check -p aphrody --locked
```

## Limites connues

- **Linker errors** sur NDK r22+ : si le linker échoue avec `unable to find library -lgcc`, c'est l'issue [rust-lang/cargo#7611](https://github.com/rust-lang/cargo/issues/7611). Workaround : utiliser cargo-ndk qui injecte les bons flags `-rtlib=compiler-rt`.
- **JNI lifetime** : les `JString` JNI doivent être convertis en `String` Rust dans le scope JNI ; ne pas leak des pointers JNI au-delà de la fn `extern "C"`.
- **Min SDK** : par défaut on cible API 28 (Android 9). Adapter via `--platform`.

## Distribuer le binaire Android

```bash
# Build pour distribution
cargo ndk --platform 28 -t aarch64-linux-android build --release -p aphrody --locked

# Le binaire se trouve dans :
ls target/aarch64-linux-android/release/cli

# Bundle dans un APK (via Android Studio ou ./gradlew :app:bundleRelease)
```

## Tests sur device

```bash
# 1. Build binaire pour ARM64
cargo ndk --platform 28 -t aarch64-linux-android build --release -p aphrody --locked

# 2. Push sur device
adb push target/aarch64-linux-android/release/cli /data/local/tmp/

# 3. Run
adb shell /data/local/tmp/cli version
```

## Références

- [Android NDK other build systems](https://developer.android.com/ndk/guides/other_build_systems)
- [cargo-ndk repo](https://github.com/bbqsrc/cargo-ndk)
- [Rust on Android — Mozilla guide](https://mozilla.github.io/firefox-browser-architecture/experiments/2017-09-21-rust-on-android.html)
