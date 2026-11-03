fn main() {
    println!("cargo:rerun-if-changed=../../packaging/windows/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let mut resource = winresource::WindowsResource::new();
    resource.set_icon("../../packaging/windows/icon.ico");
    resource.set("ProductName", "Medict");
    resource.set("FileDescription", "Offline MDict and StarDict dictionary");
    resource.set("CompanyName", "Medict contributors");
    resource.set("LegalCopyright", "Copyright (c) 2026 Medict contributors");
    resource.set(
        "FileVersion",
        env!("CARGO_PKG_VERSION").replace('.', ",") + ",0",
    );
    resource.set(
        "ProductVersion",
        env!("CARGO_PKG_VERSION").replace('.', ",") + ",0",
    );
    if let Err(error) = resource.compile() {
        panic!("failed to compile Windows resources: {error}");
    }
}
