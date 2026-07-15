fn main() {
    tauri_build::build();
    // Prefer Common Controls 6.0 (TaskDialogIndirect / SetWindowSubclass).
    // Use MANIFESTDEPENDENCY instead of embedding a second RT_MANIFEST — tauri_build
    // already embeds one into the app binary, and a duplicate causes CVT1100 / LNK1123.
    // This also covers the `cargo test --lib` harness, which otherwise has no manifest.
    println!(
        "cargo:rustc-link-arg=/MANIFESTDEPENDENCY:type='win32' \
         name='Microsoft.Windows.Common-Controls' \
         version='6.0.0.0' \
         processorArchitecture='*' \
         publicKeyToken='6595b64144ccf1df' \
         language='*'"
    );
}
