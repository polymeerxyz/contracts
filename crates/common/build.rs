use std::{path::PathBuf, process::Command};

use molecule_codegen::{Compiler, Language};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../../schemas/base.mol");
    println!("cargo:rerun-if-changed=../../schemas/distribution.mol");
    println!("cargo:rerun-if-changed=../../schemas/proof.mol");
    println!("cargo:rerun-if-changed=../../schemas/vault.mol");

    {
        let schema_path = "../../schemas/base.mol";
        let mut compiler = Compiler::new();
        let outdir = PathBuf::from("src/generated");

        compiler
            .input_schema_file(schema_path)
            .generate_code(Language::Rust)
            .output_dir(outdir)
            .run()
            .unwrap();
    }

    {
        let schema_path = "../../schemas/distribution.mol";
        let mut compiler = Compiler::new();
        let outdir = PathBuf::from("src/generated");

        compiler
            .input_schema_file(schema_path)
            .generate_code(Language::Rust)
            .output_dir(outdir)
            .run()
            .unwrap();
    }

    {
        let schema_path = "../../schemas/proof.mol";
        let mut compiler = Compiler::new();
        let outdir = PathBuf::from("src/generated");

        compiler
            .input_schema_file(schema_path)
            .generate_code(Language::Rust)
            .output_dir(outdir)
            .run()
            .unwrap();
    }

    {
        let schema_path = "../../schemas/vault.mol";
        let mut compiler = Compiler::new();
        let outdir = PathBuf::from("src/generated");

        compiler
            .input_schema_file(schema_path)
            .generate_code(Language::Rust)
            .output_dir(outdir)
            .run()
            .unwrap();
    }

    Command::new("cargo")
        .args(["fmt"])
        .status()
        .expect("failed to execute cargo fmt");
}
