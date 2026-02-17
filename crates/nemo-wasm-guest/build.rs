fn main() {
    let wit_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("wit");
    println!("cargo:metadata=wit_dir={}", wit_dir.display());
}
