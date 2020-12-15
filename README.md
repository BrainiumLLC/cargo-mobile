# cargo-mobile

*The answer to "how do I use Rust on iOS and Android?"*

cargo-mobile takes care of generating Xcode and Android Studio project files, building and running on device, generating project boilerplate, and a few other things!

Check out the [announcment post](https://dev.brainiumstudios.com/2020/11/24/cargo-mobile.html)!

## Status

Everything here works and is already used internally! However, this hasn't seen a lot of external use yet, so there could still be some rough edges.

**Building for iOS is broken on Rust 1.46.0, 1.47.0, and 1.48.0!**

Sorry for the inconvenience! This is resolved in Rust 1.49.0, but that won't be out for a lil bit. For now, you can use the current beta:

```bash
rustup update beta
rustup default beta
```

## Installation

The build will probably take a bit, so feel free to go get a snack or something.

```bash
cargo install --git https://github.com/BrainiumLLC/cargo-mobile
```

cargo-mobile is currently only supported on macOS. Adding Linux support would likely only take a small PR, but Windows support is potentially a small nightmare. (Note that only macOS can support iOS development, so other platforms could only be used for Android development!)

You'll need to have Xcode and the Android SDK/NDK installed. Some of this will ideally be automated in the future, or at least we'll provide a helpful guide and diagnostics.

Whenever you want to update:

```bash
cargo mobile update
```

### Windows

We need to tell `cargo` to not install the `cargo-apple` binary on Windows.

``` sh
cargo install --git https://github.com/BrainiumLLC/cargo-mobile --bin cargo-mobile --bin cargo-android
```

## Usage

To start a new project, all you need to do is make a directory with a cute name, `cd` into it, and then run this command:

```bash
cargo mobile init
```

> Windows: requires `Admin cmd.exe` otherwise symlinking fails :DD :/

After some straightforward prompts, you'll be asked to select a template pack. Template packs are used to generate project boilerplate, i.e. using the `bevy` template pack gives you a minimal [Bevy](https://bevyengine.org/) project that runs out-of-the-box on desktop and mobile.

| name      | info                                                                                                                              |
| --------- | --------------------------------------------------------------------------------------------------------------------------------- |
| bevy      | Minimal Bevy project derived from [sprite](https://github.com/bevyengine/bevy/blob/master/examples/2d/sprite.rs) example          |
| bevy-demo | Bevy [breakout](https://github.com/bevyengine/bevy/blob/master/examples/game/breakout.rs) demo                                    |
| wgpu      | Minimal wgpu project derived from [hello-triangle](https://github.com/gfx-rs/wgpu-rs/tree/master/examples/hello-triangle) example |
| winit     | Minimal winit project derived from [window](https://github.com/rust-windowing/winit/tree/master/examples/window) exmaple          |

**Template pack contribution is encouraged**; we'd love to have very nice template packs for Bevy, Amethyst, and whatever else people find helpful! We'll write up a guide for template pack creation soon, but in the mean time, the existing ones are a great reference point. Any template pack placed into `~./cargo-mobile/templates/apps/` will appear as an option in `cargo mobile init`.

Once you've generated your project, you can run `cargo run` as usual to run your app on desktop. However, now you can also do `cargo apple run` and `cargo android run` to run on connected iOS and Android devices respectively!

If you prefer to work in the usual IDEs, you can use `cargo apple open` and `cargo android open` to open your project in Xcode and Android Studio respectively.

For more commands, run `cargo mobile`, `cargo apple`, or `cargo android` to see help information.
