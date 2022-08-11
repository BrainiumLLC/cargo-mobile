# iced

This is just the [`iced` tour example](https://github.com/hecrj/iced/tree/master/examples/tour) with very light modifications:

- The `#[mobile_entry_point]` annotation generates all the boilerplate `extern` functions for mobile.
- Logging on Android is done using `android_logger`.

there is an issue with metal validation 
https://github.com/gfx-rs/wgpu/issues/185
which is a problem also referenced in 
https://github.com/gfx-rs/wgpu/issues/185

Until this is fixed in either iced or wgpu, metal validation needs to be deactivated, otherwise app panics.

To do so in Xcode:
Product > Scheme > Edit scheme > Run > Metal validation => disabled

To run this on desktop, just do `cargo run` like normal! For mobile, use `cargo android run` and `cargo apple run` respectively (or use `cargo android open` and `cargo apple open` to open in Android Studio and Xcode respectively).
