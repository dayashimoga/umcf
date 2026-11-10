fn main() {
    println!("cargo:rerun-if-changed=csrc/llama.h");
    println!("cargo:rerun-if-changed=csrc/ggml-backend.h");
    println!("cargo:rerun-if-changed=csrc/umcf_llama_engine.c");

    cc::Build::new()
        .include("csrc")
        .file("csrc/umcf_llama_engine.c")
        .warnings(false)
        .compile("llama");
}
