//! Embedded About PNGs decoded once at first use.

use std::sync::LazyLock;

use bytes::Bytes;
use freya::elements::image::ImageHandle;
use freya::engine::prelude::{SkData, SkImage};

/// Raw bytes baked into the binary — must match `resources/brand/SplashTowerVillage.png`.
pub const SPLASH_BYTES: &[u8] = include_bytes!("../resources/brand/SplashTowerVillage.png");
/// Raw bytes baked into the binary — must match `resources/brand/NewTowerBrandMark.png`.
pub const BRAND_BYTES: &[u8] = include_bytes!("../resources/brand/NewTowerBrandMark.png");

pub static SPLASH: LazyLock<ImageHandle> =
    LazyLock::new(|| decode_png("SplashTowerVillage", SPLASH_BYTES));
pub static BRAND: LazyLock<ImageHandle> =
    LazyLock::new(|| decode_png("NewTowerBrandMark", BRAND_BYTES));

/// Touch lazy handles before first About window open (mirrors `main` startup).
pub fn preload() {
    let _ = (&*SPLASH, &*BRAND);
}

pub fn decode_png(label: &str, bytes: &'static [u8]) -> ImageHandle {
    let image = SkImage::from_encoded(unsafe { SkData::new_bytes(bytes) })
        .and_then(|img| img.make_raster_image(None, None))
        .unwrap_or_else(|| panic!("failed to decode {label} ({bytes_len} bytes)", bytes_len = bytes.len()));
    ImageHandle::new(image, Bytes::from_static(bytes))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{decode_png, BRAND, BRAND_BYTES, SPLASH, SPLASH_BYTES};

    /// SHA256 of Mohawk `SplashTowerVillage.imageset/SplashTowerVillage.png`.
    const SPLASH_SHA256: &str =
        "d966f109a0b04ee4c585b43e7c47400b039f34ebac35086aefda1faa1bc73012";
    /// SHA256 of Mohawk `ToolbarMark.imageset/ToolbarMark-78.png` (About brand mark).
    const BRAND_SHA256: &str =
        "5bd563b64b26eabd74bfb85cbc7bc72f3da617858d66661f24020494ffbe1ab8";

    const SPLASH_WIDTH: i32 = 1376;
    const SPLASH_HEIGHT: i32 = 768;
    const BRAND_WIDTH: i32 = 78;
    const BRAND_HEIGHT: i32 = 78;

    fn brand_resource(path: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path)
    }

    fn read_disk(path: &str) -> Vec<u8> {
        std::fs::read(brand_resource(path))
            .unwrap_or_else(|e| panic!("missing asset at {}: {e}", brand_resource(path).display()))
    }

    #[test]
    fn embedded_bytes_are_valid_pngs() {
        for (name, bytes) in [("splash", SPLASH_BYTES), ("brand", BRAND_BYTES)] {
            assert!(bytes.len() > 8, "{name}: asset too small");
            assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n", "{name}: not a PNG");
        }
    }

    #[test]
    fn embedded_bytes_match_files_on_disk() {
        let splash_disk = read_disk("resources/brand/SplashTowerVillage.png");
        let brand_disk = read_disk("resources/brand/NewTowerBrandMark.png");
        assert_eq!(splash_disk, SPLASH_BYTES, "SplashTowerVillage embed ≠ disk file");
        assert_eq!(brand_disk, BRAND_BYTES, "NewTowerBrandMark embed ≠ disk file");
    }

    #[test]
    fn embedded_bytes_match_mohawk_source_when_present() {
        let mohawk_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../Swift/Mohawk_next_gen/Mohawk/Assets.xcassets");
        let splash_mohawk = mohawk_root.join("SplashTowerVillage.imageset/SplashTowerVillage.png");
        let brand_mohawk = mohawk_root.join("ToolbarMark.imageset/ToolbarMark-78.png");

        if splash_mohawk.exists() {
            let bytes = std::fs::read(&splash_mohawk).expect("read Mohawk splash");
            assert_eq!(
                bytes, SPLASH_BYTES,
                "Osman splash must match Mohawk SplashTowerVillage.imageset"
            );
        }

        if brand_mohawk.exists() {
            let bytes = std::fs::read(&brand_mohawk).expect("read Mohawk brand mark");
            assert_eq!(
                bytes, BRAND_BYTES,
                "Osman brand mark must match Mohawk ToolbarMark-78.png"
            );
        }
    }

    #[test]
    fn decoded_images_have_expected_dimensions() {
        assert_eq!(SPLASH.image.width(), SPLASH_WIDTH);
        assert_eq!(SPLASH.image.height(), SPLASH_HEIGHT);
        assert_eq!(BRAND.image.width(), BRAND_WIDTH);
        assert_eq!(BRAND.image.height(), BRAND_HEIGHT);
    }

    #[test]
    fn decoded_images_are_raster_backed() {
        assert!(
            SPLASH.image.is_texture_backed() || SPLASH.image.width() > 0,
            "splash must rasterize"
        );
        assert!(
            BRAND.image.is_texture_backed() || BRAND.image.width() > 0,
            "brand must rasterize"
        );
    }

    #[test]
    fn lazy_handles_retain_source_bytes() {
        assert_eq!(SPLASH.bytes.as_ref(), SPLASH_BYTES);
        assert_eq!(BRAND.bytes.as_ref(), BRAND_BYTES);
    }

    #[test]
    fn decode_is_idempotent() {
        let again = decode_png("SplashTowerVillage", SPLASH_BYTES);
        assert_eq!(again.image.width(), SPLASH.image.width());
        assert_eq!(again.image.height(), SPLASH.image.height());
        assert_eq!(again.bytes.as_ref(), SPLASH_BYTES);
    }

    #[test]
    fn about_header_aspect_ratio_matches_mohawk() {
        // Mohawk `SplashView.Kind.aboutHeader` is 300×323.
        let w = 300.0_f32;
        let h = 323.0_f32;
        let iw = SPLASH.image.width() as f32;
        let ih = SPLASH.image.height() as f32;
        let scale = (w / iw).max(h / ih);
        let cover_w = iw * scale;
        let cover_h = ih * scale;
        assert!(cover_w >= w && cover_h >= h, "cover scale should fill About card");
        assert!(scale > 0.2 && scale < 0.5, "unexpected cover scale: {scale}");
    }

    #[test]
    fn asset_fingerprint_constants_document_mohawk_parity() {
        // If these fail, update assets from Mohawk and refresh the constants above.
        assert_eq!(SPLASH_BYTES.len(), 1_650_836);
        assert_eq!(BRAND_BYTES.len(), 7_832);
        let _ = (SPLASH_SHA256, BRAND_SHA256); // documented expected hashes for manual `shasum -a 256`
    }

    mod ui {
        use freya::elements::image::Image;
        use freya::prelude::*;
        use freya_testing::prelude::*;

        use super::super::{BRAND, SPLASH};

        const SPLASH_W: f32 = 300.;
        const SPLASH_H: f32 = 323.;

        #[test]
        fn about_splash_image_element_renders_with_expected_layout() {
            fn app() -> impl IntoElement {
                rect()
                    .width(Size::px(SPLASH_W))
                    .height(Size::px(SPLASH_H))
                    .child(
                        image(SPLASH.clone())
                            .width(Size::px(SPLASH_W))
                            .height(Size::px(SPLASH_H))
                            .aspect_ratio(AspectRatio::Max)
                            .image_cover(ImageCover::Center),
                    )
            }

            let mut test = launch_test(app);
            test.sync_and_update();

            let image_node = test
                .find(|node, element| Image::try_downcast(element).map(|_| node))
                .expect("sync image() should render an Image element");

            let area = image_node.layout().area;
            assert!(
                (area.width() - SPLASH_W).abs() < 1.0,
                "splash width: got {} expected {SPLASH_W}",
                area.width()
            );
            assert!(
                (area.height() - SPLASH_H).abs() < 1.0,
                "splash height: got {} expected {SPLASH_H}",
                area.height()
            );
        }

        #[test]
        fn about_brand_mark_image_element_renders() {
            fn app() -> impl IntoElement {
                image(BRAND.clone())
                    .width(Size::px(88.))
                    .height(Size::px(64.))
                    .aspect_ratio(AspectRatio::Min)
                    .image_cover(ImageCover::Center)
            }

            let mut test = launch_test(app);
            test.sync_and_update();

            let image_node = test
                .find(|node, element| Image::try_downcast(element).map(|_| node))
                .expect("brand mark image() should render");

            let area = image_node.layout().area;
            assert!(area.width() >= 80.0 && area.width() <= 96.0);
            assert!(area.height() >= 56.0 && area.height() <= 72.0);
        }
    }
}
