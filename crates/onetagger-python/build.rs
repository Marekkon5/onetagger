fn main() {
    //! Important, fixes python modules
    println!("cargo:rustc-link-arg=-Wl,-export-dynamic");
}
