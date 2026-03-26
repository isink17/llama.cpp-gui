use std::env;
use std::fs;
use std::path::PathBuf;

fn ensure_icon_ico() -> Result<(), String> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .map_err(|err| format!("CARGO_MANIFEST_DIR is not set: {err}"))?;
    let icons_dir = PathBuf::from(manifest_dir).join("icons");
    let icon_ico = icons_dir.join("icon.ico");

    if icon_ico.exists() {
        return Ok(());
    }

    fs::create_dir_all(&icons_dir)
        .map_err(|err| format!("failed to create {}: {err}", icons_dir.display()))?;

    // Minimal valid 1x1 32bpp ICO (ICONDIR + ICONDIRENTRY + BITMAPINFOHEADER + pixel + mask).
    // This is only a fallback for CI environments where icon assets were not checked out correctly.
    let ico_bytes: [u8; 70] = [
        0x00, 0x00, 0x01, 0x00, 0x01, 0x00, // ICONDIR
        0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x20, 0x00, 0x30, 0x00, 0x00, 0x00, 0x16, 0x00, 0x00,
        0x00, // ICONDIRENTRY
        0x28, 0x00, 0x00, 0x00, // BITMAPINFOHEADER size
        0x01, 0x00, 0x00, 0x00, // width
        0x02, 0x00, 0x00, 0x00, // height (xor+and)
        0x01, 0x00, // planes
        0x20, 0x00, // bit count
        0x00, 0x00, 0x00, 0x00, // compression
        0x08, 0x00, 0x00, 0x00, // image size
        0x00, 0x00, 0x00, 0x00, // x ppm
        0x00, 0x00, 0x00, 0x00, // y ppm
        0x00, 0x00, 0x00, 0x00, // clr used
        0x00, 0x00, 0x00, 0x00, // clr important
        0xFF, 0x71, 0x2C, 0xFF, // pixel BGRA
        0x00, 0x00, 0x00, 0x00, // AND mask row (aligned to 4 bytes)
    ];

    fs::write(&icon_ico, ico_bytes)
        .map_err(|err| format!("failed to write {}: {err}", icon_ico.display()))?;
    println!(
        "cargo:warning=generated fallback icon at {}",
        icon_ico.display()
    );
    Ok(())
}

fn main() {
    if let Err(err) = ensure_icon_ico() {
        panic!("{err}");
    }
    tauri_build::build()
}
