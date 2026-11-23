// The web UI is embedded into the binary from `web/build`.
// Create the folder when it is missing so that a backend-only build still compiles.
fn main() {
    let dir = std::path::Path::new("web/build");
    if !dir.exists() {
        std::fs::create_dir_all(dir).expect("create web/build");
    }
    println!("cargo:rerun-if-changed=web/build");
}
