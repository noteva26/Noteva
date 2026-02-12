//! CLI tool to generate hook contract documentation from hook-registry.json.
//!
//! Usage: `cargo run --bin generate-hook-docs`
//!
//! Outputs to `docs/hook-contracts.md`.

use std::fs;
use std::path::Path;

use noteva::plugin::hook_registry::HookRegistry;
use noteva::plugin::doc_gen::generate_hook_docs;

fn main() {
    let registry = HookRegistry::load_embedded();
    let markdown = generate_hook_docs(&registry);

    let out_dir = Path::new("docs");
    if !out_dir.exists() {
        fs::create_dir_all(out_dir).expect("failed to create docs directory");
    }

    let out_path = out_dir.join("hook-contracts.md");
    fs::write(&out_path, &markdown).expect("failed to write hook-contracts.md");

    println!("Generated {} ({} bytes)", out_path.display(), markdown.len());
}
