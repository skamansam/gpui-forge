//! The Forge Runtime.
//!
//! Compiled once per base component system. Select the base with exactly one
//! `base-*` Cargo feature; `base-gpui-kit` is the default.

#[cfg(not(any(
    feature = "base-gpui-kit",
    feature = "base-bezel",
    feature = "base-adabraka",
    feature = "base-gpui02"
)))]
compile_error!(
    "gpui-forge-runtime: enable exactly one base feature: \
     base-gpui-kit, base-bezel, base-adabraka or base-gpui02"
);

#[cfg(any(
    all(feature = "base-gpui-kit", feature = "base-bezel"),
    all(feature = "base-gpui-kit", feature = "base-adabraka"),
    all(feature = "base-gpui-kit", feature = "base-gpui02"),
    all(feature = "base-bezel", feature = "base-adabraka"),
    all(feature = "base-bezel", feature = "base-gpui02"),
    all(feature = "base-adabraka", feature = "base-gpui02"),
))]
compile_error!(
    "gpui-forge-runtime: base features are mutually exclusive (each base links its own GPUI). \
     Use `default-features = false` when selecting a non-default base."
);

#[cfg(feature = "base-bezel")]
compile_error!("gpui-forge-runtime: the Bezel base is not implemented yet (#94)");

#[cfg(feature = "base-adabraka")]
compile_error!("gpui-forge-runtime: the adabraka-ui base is not implemented yet (#95)");

#[cfg(feature = "base-gpui02")]
compile_error!("gpui-forge-runtime: the Vanilla GPUI 0.2 base is not implemented yet (#96)");

pub mod base;

pub use forge_spec;
