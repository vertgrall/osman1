use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::process::Command;

use icns::{IconFamily, Image};

fn main() {
    write_git_metadata();
    build_app_icns();
}

fn write_git_metadata() {
    let branch = git_output(&["rev-parse", "--abbrev-ref", "HEAD"]).unwrap_or_else(|| "unknown".into());
    let short_hash = git_output(&["rev-parse", "--short", "HEAD"]).unwrap_or_else(|| "unknown".into());

    let out = Path::new(&std::env::var("OUT_DIR").unwrap()).join("git_metadata.rs");
    std::fs::write(
        &out,
        format!(
            r#"pub const BRANCH: &str = "{branch}";
pub const SHORT_HASH: &str = "{short_hash}";
"#
        ),
    )
    .expect("write git metadata");

    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads");
}

fn build_app_icns() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let root = Path::new(&manifest);
    let master = root.join("resources/icon/OsmanAppIcon-1024.png");
    let iconset = root.join("resources/icon/AppIcon.appiconset");
    let out = root.join("resources/icon/AppIcon.icns");

    println!("cargo:rerun-if-changed={}", master.display());
    println!("cargo:rerun-if-changed={}", iconset.join("Contents.json").display());

    if !master.exists() {
        return;
    }

    ensure_iconset_pngs(&master, &iconset);

    let mut family = IconFamily::new();
    for name in [
        "Icon-16.png",
        "Icon-32.png",
        "Icon-64.png",
        "Icon-128.png",
        "Icon-256.png",
        "Icon-512.png",
        "Icon-1024.png",
    ] {
        let path = iconset.join(name);
        if !path.exists() {
            continue;
        }
        let file = BufReader::new(File::open(&path).unwrap_or_else(|e| panic!("open {}: {e}", path.display())));
        let image = Image::read_png(file).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        family
            .add_icon(&image)
            .unwrap_or_else(|e| panic!("add {}: {e}", path.display()));
        println!("cargo:rerun-if-changed={}", path.display());
    }

    let file = File::create(&out).unwrap_or_else(|e| panic!("create {}: {e}", out.display()));
    family
        .write(std::io::BufWriter::new(file))
        .unwrap_or_else(|e| panic!("write {}: {e}", out.display()));
}

fn ensure_iconset_pngs(master: &Path, iconset: &Path) {
    std::fs::create_dir_all(iconset).expect("create iconset dir");

    let sizes = [
        (16, "Icon-16.png"),
        (32, "Icon-32.png"),
        (64, "Icon-64.png"),
        (128, "Icon-128.png"),
        (256, "Icon-256.png"),
        (512, "Icon-512.png"),
    ];

    for (size, name) in sizes {
        let out = iconset.join(name);
        if out.exists() {
            continue;
        }
        resize_png(master, size, size, &out);
    }

    let master_copy = iconset.join("Icon-1024.png");
    if !master_copy.exists() {
        std::fs::copy(master, &master_copy).expect("copy 1024 master");
    }

    let menubar = master
        .parent()
        .expect("icon dir")
        .join("MenubarIcon-22.png");
    if !menubar.exists() {
        resize_png(master, 22, 22, &menubar);
    }

    let window = master
        .parent()
        .expect("icon dir")
        .join("WindowIcon-128.png");
    if !window.exists() {
        resize_png(master, 128, 128, &window);
    }
}

fn resize_png(src: &Path, width: u32, height: u32, dst: &Path) {
    let status = Command::new("sips")
        .args([
            "-z",
            &height.to_string(),
            &width.to_string(),
            src.to_str().expect("utf8 path"),
            "--out",
            dst.to_str().expect("utf8 path"),
        ])
        .status()
        .unwrap_or_else(|e| panic!("run sips for {}: {e}", dst.display()));

    if !status.success() {
        panic!("sips failed for {}", dst.display());
    }
}

fn git_output(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
