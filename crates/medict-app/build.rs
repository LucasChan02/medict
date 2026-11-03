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
    let numeric_version = format!("{},0", env!("CARGO_PKG_VERSION").replace('.', ","));
    resource.set("FileVersion", &numeric_version);
    resource.set("ProductVersion", &numeric_version);
    if let Err(error) = resource.compile() {
        panic!("failed to compile Windows resources: {error}");
    }
}
