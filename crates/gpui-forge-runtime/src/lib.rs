//! The Forge Runtime.
//!
//! Compiled once per base component system. Select the base with exactly one
//! `base-*` Cargo feature; `base-gpui-kit` is the default.

const _: () = {
    let bases = cfg!(feature = "base-gpui-kit") as usize
        + cfg!(feature = "base-bezel") as usize
        + cfg!(feature = "base-adabraka") as usize
        + cfg!(feature = "base-gpui02") as usize
        + cfg!(feature = "base-ratatui") as usize
        + cfg!(feature = "base-iced") as usize;
    assert!(
        bases == 1,
        "gpui-forge-runtime: enable exactly one base feature: \
         base-gpui-kit, base-bezel, base-adabraka, base-gpui02, base-ratatui or base-iced. \
         Use `default-features = false` when selecting a non-default base."
    );
};

#[cfg(feature = "base-ratatui")]
compile_error!("gpui-forge-runtime: the ratatui (TUI) base is not implemented yet (#112)");

#[cfg(feature = "base-iced")]
compile_error!("gpui-forge-runtime: the iced base is not implemented yet (#113)");

pub mod base;
pub mod check;
#[cfg(feature = "base-adabraka")]
compile_error!(
    "gpui-forge-runtime: the adabraka-ui base needs its own workspace — adabraka-ui's tree-sitter 0.25 hard-dep conflicts with gpui-kit's 0.26 via links (#95)"
);

#[cfg(any(
    feature = "base-gpui-kit",
    feature = "base-gpui02",
    feature = "base-bezel"
))]
pub mod render;

pub use forge_spec;
