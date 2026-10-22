fn main() {
    println!("cargo:rerun-if-changed=assets/cinematch.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winres::WindowsResource::new();
    res.set_icon("assets/cinematch.ico");
    res.set("ProductName", "CineMatch");
    res.set("FileDescription", "CineMatch - esplorazione cinematografica");
    res.set("CompanyName", "Emanuele Paris");
    res.set("LegalCopyright", "Copyright (C) 2026 Emanuele Paris");
    let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_default();
    res.set("ProductVersion", &version);
    res.set("FileVersion", &version);
    if let Err(e) = res.compile() {
        println!("cargo:warning=icona eseguibile non incorporata: {e}");
        return;
    }
    // Con il linker GNU una libreria statica di risorse non viene inclusa
    // se nessun simbolo la richiama: forziamo l'archivio generato da winres.
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu") {
        let out = std::env::var("OUT_DIR").unwrap_or_default();
        println!("cargo:rustc-link-arg=-Wl,--whole-archive");
        println!("cargo:rustc-link-arg={out}/libresource.a");
        println!("cargo:rustc-link-arg=-Wl,--no-whole-archive");
    }
}
