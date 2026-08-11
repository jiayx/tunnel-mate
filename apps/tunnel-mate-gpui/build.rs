fn main() {
    const WINDOWS_ICON: &str = "../../assets/icons/icon.ico";

    println!("cargo:rerun-if-changed={WINDOWS_ICON}");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon(WINDOWS_ICON)
            .compile()
            .expect("failed to embed the Windows application icon");
    }
}
