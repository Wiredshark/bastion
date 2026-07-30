//! `APEX-T9.3` — runs the certificate generator against the real, live
//! tree and writes the rendered result. Not merely runnable: this is
//! what actually RUNS it. Every number in the output comes from
//! `veloren_server::apex_certificate::all_roots_v1`/`all_attestations_v1`
//! at the moment this binary executes -- nothing here hand-writes a
//! property, a root, or an open case.
//!
//! Usage: `cargo run --bin apex_certificate_gen -- <output-path>`

fn main() {
    let output_path = std::env::args().nth(1).expect("usage: apex_certificate_gen <output-path>");

    let roots = veloren_server::apex_certificate::all_roots_v1();
    let attestations = veloren_server::apex_certificate::all_attestations_v1();
    let certificate = common::apex::certificate::generate_certificate_v1(&roots, &attestations);
    let rendered = common::apex::certificate::render_certificate_v1(&certificate);

    std::fs::write(&output_path, &rendered).unwrap_or_else(|e| panic!("failed to write {output_path}: {e}"));
    println!("wrote {} bytes to {output_path}", rendered.len());
}
