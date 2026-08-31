fn main() {
    println!("cargo:rerun-if-env-changed=TARGET");
    println!(
        "cargo:rustc-env=NEOMACS_VIDEO_HOST_TRIPLE={}",
        std::env::var("TARGET").expect("Cargo always sets TARGET for build scripts")
    );
}
