fn main() {
    println!(
        "forge {} (spec {})",
        env!("CARGO_PKG_VERSION"),
        forge_spec::SPEC_VERSION
    );
}
