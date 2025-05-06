use codegen::{Compiler, Language};
use std::{path, process::Command};

fn main() {
    let schema_path = "../../schemas/data.mol";
    let mut compiler = Compiler::new();
    let outdir = path::PathBuf::from("src/generated");

    compiler
        .input_schema_file(schema_path)
        .generate_code(Language::Rust)
        .output_dir(outdir)
        .run()
        .unwrap();

    Command::new("cargo")
        .args(["fmt"])
        .status()
        .expect("failed to execute cargo fmt");
}
