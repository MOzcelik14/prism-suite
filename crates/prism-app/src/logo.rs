// Copyright (C) 2026 Murat Özçelik <mozcelik14>
// SPDX-License-Identifier: GPL-3.0-only

//! Resolution-independent rasterization of the prisM. SVG logo.

/// Embedded vector source of the suite logo.
pub const LOGO_SVG: &str = include_str!("../../../assets/logo.svg");

/// Rasterizes the logo to an exact `px` x `px` RGBA image (premultiplied alpha).
pub fn render_logo_rgba(px: u32) -> Option<Vec<u8>> {
    let px = px.max(8);
    let tree = resvg::usvg::Tree::from_str(LOGO_SVG, &resvg::usvg::Options::default()).ok()?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(px, px)?;
    let size = tree.size();
    let scale = px as f32 / size.width().max(size.height());
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    Some(pixmap.take())
}

/// Rasterizes the logo into an egui image of `px` x `px` pixels.
pub fn render_logo_image(px: u32) -> Option<egui::ColorImage> {
    let px = px.max(8);
    let data = render_logo_rgba(px)?;
    Some(egui::ColorImage::from_rgba_premultiplied(
        [px as usize, px as usize],
        &data,
    ))
}

#[cfg(test)]
mod tests {
    #[test]
    fn renders_logo_at_multiple_sizes() {
        for px in [16u32, 36, 72, 256] {
            let data = super::render_logo_rgba(px).expect("render");
            assert_eq!(data.len(), (px * px * 4) as usize);
            assert!(data.chunks(4).any(|p| p[3] > 0));
        }
        if let Ok(path) = std::env::var("PRISM_LOGO_DUMP") {
            let data = super::render_logo_rgba(512).unwrap();
            let mut out = b"P6 512 512 255\n".to_vec();
            for p in data.chunks(4) {
                out.extend_from_slice(&p[..3]);
            }
            std::fs::write(path, out).unwrap();
        }
    }
}
