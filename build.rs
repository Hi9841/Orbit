fn main() {
    println!("cargo:rerun-if-changed=assets/orbit.ico");
    println!("cargo:rerun-if-changed=assets/orbit.manifest");
    println!("cargo:rerun-if-changed=packaging/update-public-key.txt");
    println!("cargo:rerun-if-env-changed=ORBIT_UPDATE_MANIFEST_URL");
    println!("cargo:rerun-if-env-changed=ORBIT_UPDATE_PUBLIC_KEY");
    println!("cargo:rerun-if-env-changed=ORBIT_DEVELOPMENT_RELEASES_URL");
    let development_feed = std::env::var("ORBIT_DEVELOPMENT_RELEASES_URL").unwrap_or_else(|_| {
        "https://api.github.com/repos/Hi9841/Orbit/releases?per_page=10".into()
    });
    println!("cargo:rustc-env=ORBIT_DEVELOPMENT_RELEASES_URL={development_feed}");
    let manifest = std::env::var("ORBIT_UPDATE_MANIFEST_URL").unwrap_or_else(|_| {
        "https://github.com/Hi9841/Orbit/releases/latest/download/update.json".into()
    });
    let public_key = std::env::var("ORBIT_UPDATE_PUBLIC_KEY").unwrap_or_else(|_| {
        std::fs::read_to_string("packaging/update-public-key.txt")
            .expect("update public key is missing")
            .trim()
            .to_owned()
    });
    println!("cargo:rustc-env=ORBIT_UPDATE_MANIFEST_URL={manifest}");
    println!("cargo:rustc-env=ORBIT_UPDATE_PUBLIC_KEY={public_key}");
    if cfg!(target_os = "windows") {
        winres::WindowsResource::new()
            .set_icon("assets/orbit.ico")
            .set_manifest_file("assets/orbit.manifest")
            .compile()
            .expect("failed to embed Orbit icon");
    }
}
