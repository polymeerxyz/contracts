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

        compiler
            .input_schema_file(schema_path)
            .generate_code(Language::Rust)
            .output_dir_set_default()
            .run()
            .unwrap();
    }

    {
        let schema_path = "../../schemas/distribution.mol";
        let mut compiler = Compiler::new();

        compiler
            .input_schema_file(schema_path)
            .generate_code(Language::Rust)
            .output_dir_set_default()
            .run()
            .unwrap();
    }

    {
        let schema_path = "../../schemas/proof.mol";
        let mut compiler = Compiler::new();

        compiler
            .input_schema_file(schema_path)
            .generate_code(Language::Rust)
            .output_dir_set_default()
            .run()
            .unwrap();
    }

    {
        let schema_path = "../../schemas/vault.mol";
        let mut compiler = Compiler::new();

        compiler
            .input_schema_file(schema_path)
            .generate_code(Language::Rust)
            .output_dir_set_default()
            .run()
            .unwrap();
    }
}
