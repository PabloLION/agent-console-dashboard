//! Build script for napi-rs bindings.
//!
//! This is required for the napi feature to generate correct bindings.

fn main() {
    #[cfg(feature = "napi")]
    napi_build::setup();
}
