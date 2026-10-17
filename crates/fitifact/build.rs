fn main() {
    // Tauri names shipped sidecars `<tool>-<target triple>`; media_tools accepts only that name.
    println!(
        "cargo:rustc-env=FITIFACT_TARGET_TRIPLE={}",
        std::env::var("TARGET").expect("cargo sets TARGET for build scripts")
    );
    println!("cargo:rerun-if-changed=build.rs");
}
