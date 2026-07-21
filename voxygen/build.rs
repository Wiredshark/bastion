// DET-BLD-010 (v6 deep-pass, Critical): the DECISION branches on the TARGET
// (CARGO_CFG_TARGET_OS), not the host — the old cfg(windows) answered
// 'is the build machine Windows', so cross-compilation silently included or
// skipped the resource step based on host identity. The winres TOOL is
// host-gated by Cargo ([target.cfg(windows).build-dependencies]), so a
// non-Windows host targeting Windows gets a LOUD warning instead of a
// silent skip.
fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "windows" {
        #[cfg(windows)]
        {
            // Set executable logo with winres:
            let mut res = winres::WindowsResource::new();
            res.set_icon("../assets/voxygen/logo.ico");
            res.compile().expect("failed to build executable logo.");
        }
        #[cfg(not(windows))]
        println!(
            "cargo:warning=DET-BLD-010: targeting windows from a non-windows host —              winres unavailable, executable will lack the icon resource"
        );
    }
}
