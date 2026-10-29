use egui::{epaint::PathShape, Color32, Painter, Pos2, Rect, Rounding, Stroke};
use prism_core::catalog::AppId;

/// Renders a vector icon for an application or the suite with the capital "M" and trailing-dot motif.
pub fn draw_app_icon(painter: &Painter, rect: Rect, app_id: AppId) {
    let center = rect.center();
    let size = rect.width().min(rect.height());
    let corner_radius = size * 0.22_f32;

    // Background card for icon
    let (bg_color, border_color, accent_color) = match app_id {
        AppId::Trim => (
            Color32::from_rgb(15, 23, 42),   // Slate 900
            Color32::from_rgb(56, 189, 248), // Sky 400
            Color32::from_rgb(56, 189, 248),
        ),
        AppId::Stem => (
            Color32::from_rgb(6, 30, 23),    // Dark Emerald
            Color32::from_rgb(52, 211, 153), // Emerald 400
            Color32::from_rgb(52, 211, 153),
        ),
        AppId::Beam => (
            Color32::from_rgb(26, 16, 40),    // Dark Violet
            Color32::from_rgb(192, 132, 252), // Purple 400
            Color32::from_rgb(192, 132, 252),
        ),
    };

    // Draw background rounded rect
    painter.rect_filled(rect, Rounding::same(corner_radius), bg_color);
    painter.rect_stroke(
        rect,
        Rounding::same(corner_radius),
        Stroke::new(1.5_f32, border_color.linear_multiply(0.6_f32)),
    );

    let m_width = size * 0.52_f32;
    let m_height = size * 0.44_f32;
    let m_top = center.y - m_height * 0.55_f32;
    let m_bottom = m_top + m_height;
    let m_left = center.x - m_width * 0.58_f32;
    let m_right = m_left + m_width;
    let m_mid = (m_left + m_right) * 0.5_f32;

    // The signature bold geometric "M"
    let stroke_width = size * 0.08_f32;
    let stroke = Stroke::new(stroke_width, Color32::from_rgb(240, 243, 246));

    // Outer strokes of M
    painter.line_segment(
        [Pos2::new(m_left, m_bottom), Pos2::new(m_left, m_top)],
        stroke,
    );
    painter.line_segment(
        [
            Pos2::new(m_left, m_top),
            Pos2::new(m_mid, m_bottom * 0.85_f32 + m_top * 0.15_f32),
        ],
        stroke,
    );
    painter.line_segment(
        [
            Pos2::new(m_mid, m_bottom * 0.85_f32 + m_top * 0.15_f32),
            Pos2::new(m_right, m_top),
        ],
        stroke,
    );
    painter.line_segment(
        [Pos2::new(m_right, m_top), Pos2::new(m_right, m_bottom)],
        stroke,
    );

    // App-specific vector motif inside the icon
    match app_id {
        AppId::Trim => {
            // Video film strip & playback triangle
            let play_center = Pos2::new(m_mid, m_top + m_height * 0.35_f32);
            let tri_size = size * 0.10_f32;
            let p1 = Pos2::new(
                play_center.x - tri_size * 0.6_f32,
                play_center.y - tri_size * 0.7_f32,
            );
            let p2 = Pos2::new(play_center.x + tri_size * 0.7_f32, play_center.y);
            let p3 = Pos2::new(
                play_center.x - tri_size * 0.6_f32,
                play_center.y + tri_size * 0.7_f32,
            );
            let shape = PathShape::convex_polygon(vec![p1, p2, p3], accent_color, Stroke::NONE);
            painter.add(shape);
        }
        AppId::Stem => {
            // Audio wave bars
            let bar_w = size * 0.035_f32;
            let b1 = Pos2::new(m_mid - size * 0.06_f32, m_top + m_height * 0.35_f32);
            let b2 = Pos2::new(m_mid, m_top + m_height * 0.20_f32);
            let b3 = Pos2::new(m_mid + size * 0.06_f32, m_top + m_height * 0.32_f32);

            painter.line_segment(
                [
                    Pos2::new(b1.x, b1.y - size * 0.08_f32),
                    Pos2::new(b1.x, b1.y + size * 0.08_f32),
                ],
                Stroke::new(bar_w, accent_color),
            );
            painter.line_segment(
                [
                    Pos2::new(b2.x, b2.y - size * 0.13_f32),
                    Pos2::new(b2.x, b2.y + size * 0.13_f32),
                ],
                Stroke::new(bar_w, accent_color),
            );
            painter.line_segment(
                [
                    Pos2::new(b3.x, b3.y - size * 0.07_f32),
                    Pos2::new(b3.x, b3.y + size * 0.07_f32),
                ],
                Stroke::new(bar_w, accent_color),
            );
        }
        AppId::Beam => {
            // Photo light ray / aperture ring
            let ring_center = Pos2::new(m_mid, m_top + m_height * 0.32_f32);
            painter.circle_stroke(
                ring_center,
                size * 0.10_f32,
                Stroke::new(size * 0.03_f32, accent_color),
            );
            painter.circle_filled(ring_center, size * 0.04_f32, accent_color);
        }
    }

    // Trailing dot motif: distinct glowing dot '.' at the bottom right of the M
    let dot_radius = size * 0.06_f32;
    let dot_center = Pos2::new(
        m_right + dot_radius * 1.6_f32,
        m_bottom - dot_radius * 0.5_f32,
    );

    // Outer glow
    painter.circle_filled(
        dot_center,
        dot_radius * 1.6_f32,
        accent_color.linear_multiply(0.25_f32),
    );
    // Core dot
    painter.circle_filled(dot_center, dot_radius, accent_color);
}

/// Renders the signature prisM. Suite logo.
pub fn draw_suite_logo(painter: &Painter, rect: Rect) {
    let center = rect.center();
    let size = rect.width().min(rect.height());
    let corner_radius = size * 0.22_f32;

    // Prismatic dark background
    let bg_color = Color32::from_rgb(18, 18, 24);
    painter.rect_filled(rect, Rounding::same(corner_radius), bg_color);
    painter.rect_stroke(
        rect,
        Rounding::same(corner_radius),
        Stroke::new(
            1.5_f32,
            Color32::from_rgb(99, 102, 241).linear_multiply(0.5_f32),
        ),
    );

    // Prismatic triangle
    let tri_top = Pos2::new(center.x - size * 0.05_f32, center.y - size * 0.28_f32);
    let tri_left = Pos2::new(center.x - size * 0.32_f32, center.y + size * 0.22_f32);
    let tri_right = Pos2::new(center.x + size * 0.22_f32, center.y + size * 0.22_f32);

    painter.line_segment(
        [tri_top, tri_left],
        Stroke::new(size * 0.04_f32, Color32::from_rgb(56, 189, 248)),
    );
    painter.line_segment(
        [tri_left, tri_right],
        Stroke::new(size * 0.04_f32, Color32::from_rgb(52, 211, 153)),
    );
    painter.line_segment(
        [tri_right, tri_top],
        Stroke::new(size * 0.04_f32, Color32::from_rgb(244, 63, 94)),
    );

    // Bold "M"
    let m_width = size * 0.44_f32;
    let m_height = size * 0.34_f32;
    let m_top = center.y - m_height * 0.4_f32;
    let m_bottom = m_top + m_height;
    let m_left = center.x - m_width * 0.5_f32;
    let m_right = m_left + m_width;
    let m_mid = (m_left + m_right) * 0.5_f32;

    let stroke = Stroke::new(size * 0.07_f32, Color32::from_rgb(255, 255, 255));
    painter.line_segment(
        [Pos2::new(m_left, m_bottom), Pos2::new(m_left, m_top)],
        stroke,
    );
    painter.line_segment(
        [
            Pos2::new(m_left, m_top),
            Pos2::new(m_mid, m_bottom * 0.85_f32 + m_top * 0.15_f32),
        ],
        stroke,
    );
    painter.line_segment(
        [
            Pos2::new(m_mid, m_bottom * 0.85_f32 + m_top * 0.15_f32),
            Pos2::new(m_right, m_top),
        ],
        stroke,
    );
    painter.line_segment(
        [Pos2::new(m_right, m_top), Pos2::new(m_right, m_bottom)],
        stroke,
    );

    // Trailing dot motif
    let dot_radius = size * 0.06_f32;
    let dot_center = Pos2::new(
        m_right + dot_radius * 1.5_f32,
        m_bottom - dot_radius * 0.5_f32,
    );
    painter.circle_filled(
        dot_center,
        dot_radius * 1.6_f32,
        Color32::from_rgb(244, 63, 94).linear_multiply(0.3_f32),
    );
    painter.circle_filled(dot_center, dot_radius, Color32::from_rgb(244, 63, 94));
}
