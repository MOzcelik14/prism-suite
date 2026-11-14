// Copyright (C) 2026 Murat Özçelik <mozcelik14>
// SPDX-License-Identifier: GPL-3.0-only

use egui::{pos2, vec2, Color32, Painter, Pos2, Rect, Stroke};

/// Draws a crisp status dot (solid or hollow ring).
pub fn draw_status_dot(painter: &Painter, center: Pos2, radius: f32, color: Color32, filled: bool) {
    if filled {
        painter.circle_filled(center, radius, color);
    } else {
        painter.circle_stroke(center, radius, Stroke::new(1.4_f32, color));
    }
}

/// Draws a 2x2 grid icon for the Applications navigation item.
pub fn draw_apps_grid_icon(painter: &Painter, rect: Rect, color: Color32) {
    let s = rect.width().min(rect.height());
    let c = rect.center();
    let half = s * 0.36;
    let gap = s * 0.08;
    let box_sz = half - gap * 0.5;
    let r = 1.5;

    let tl = Rect::from_min_size(pos2(c.x - half, c.y - half), vec2(box_sz, box_sz));
    let tr = Rect::from_min_size(pos2(c.x + gap * 0.5, c.y - half), vec2(box_sz, box_sz));
    let bl = Rect::from_min_size(pos2(c.x - half, c.y + gap * 0.5), vec2(box_sz, box_sz));
    let br = Rect::from_min_size(pos2(c.x + gap * 0.5, c.y + gap * 0.5), vec2(box_sz, box_sz));

    let stroke = Stroke::new(1.4_f32, color);
    painter.rect_stroke(tl, r, stroke);
    painter.rect_stroke(tr, r, stroke);
    painter.rect_stroke(bl, r, stroke);
    painter.rect_stroke(br, r, stroke);
}

/// Draws a clean document/lines icon for the What's New navigation item.
pub fn draw_notes_icon(painter: &Painter, rect: Rect, color: Color32) {
    let s = rect.width().min(rect.height());
    let c = rect.center();
    let w = s * 0.68;
    let h = s * 0.76;
    let box_rect = Rect::from_center_size(c, vec2(w, h));
    let stroke = Stroke::new(1.4_f32, color);
    painter.rect_stroke(box_rect, 2.0, stroke);

    let left = box_rect.left() + w * 0.22;
    let right = box_rect.right() - w * 0.22;
    let y1 = box_rect.top() + h * 0.32;
    let y2 = box_rect.top() + h * 0.52;
    let y3 = box_rect.top() + h * 0.72;
    painter.line_segment([pos2(left, y1), pos2(right, y1)], stroke);
    painter.line_segment([pos2(left, y2), pos2(right, y2)], stroke);
    painter.line_segment(
        [pos2(left, y3), pos2(left + (right - left) * 0.65, y3)],
        stroke,
    );
}

/// Draws a clean north-east external link arrow.
pub fn draw_external_arrow(painter: &Painter, rect: Rect, color: Color32) {
    let s = rect.width().min(rect.height());
    let c = rect.center();
    let d = s * 0.26;
    let stroke = Stroke::new(1.5_f32, color);

    let bl = pos2(c.x - d, c.y + d);
    let tr = pos2(c.x + d, c.y - d);
    painter.line_segment([bl, tr], stroke);
    painter.line_segment([pos2(tr.x - d * 1.05, tr.y), tr], stroke);
    painter.line_segment([tr, pos2(tr.x, tr.y + d * 1.05)], stroke);
}

/// Draws a clean circular refresh arrow.
pub fn draw_refresh_icon(painter: &Painter, rect: Rect, color: Color32) {
    let s = rect.width().min(rect.height());
    let c = rect.center();
    let r = s * 0.32;
    let stroke = Stroke::new(1.5_f32, color);

    // Draw ~270 degree arc using 12 segments
    let steps = 12;
    let start_angle = -0.3_f32;
    let end_angle = 4.3_f32;
    let mut prev = pos2(c.x + r * start_angle.cos(), c.y + r * start_angle.sin());
    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        let a = start_angle + (end_angle - start_angle) * t;
        let next = pos2(c.x + r * a.cos(), c.y + r * a.sin());
        painter.line_segment([prev, next], stroke);
        prev = next;
    }

    // Arrowhead at start_angle
    let tip = pos2(c.x + r * start_angle.cos(), c.y + r * start_angle.sin());
    painter.line_segment([tip, pos2(tip.x - s * 0.16, tip.y - s * 0.04)], stroke);
    painter.line_segment([tip, pos2(tip.x + s * 0.03, tip.y - s * 0.17)], stroke);
}

/// Draws a clean sliders/settings icon (two horizontal tracks with knobs).
pub fn draw_settings_icon(painter: &Painter, rect: Rect, color: Color32) {
    let s = rect.width().min(rect.height());
    let c = rect.center();
    let left = c.x - s * 0.34;
    let right = c.x + s * 0.34;
    let y1 = c.y - s * 0.16;
    let y2 = c.y + s * 0.16;
    let stroke = Stroke::new(1.4_f32, color);

    painter.line_segment([pos2(left, y1), pos2(right, y1)], stroke);
    painter.line_segment([pos2(left, y2), pos2(right, y2)], stroke);
    painter.circle_filled(pos2(c.x + s * 0.12, y1), s * 0.10, color);
    painter.circle_filled(pos2(c.x - s * 0.12, y2), s * 0.10, color);
}

/// Draws a clean circle-i info icon.
pub fn draw_info_icon(painter: &Painter, rect: Rect, color: Color32) {
    let s = rect.width().min(rect.height());
    let c = rect.center();
    let r = s * 0.36;
    let stroke = Stroke::new(1.4_f32, color);

    painter.circle_stroke(c, r, stroke);
    painter.circle_filled(pos2(c.x, c.y - r * 0.42), s * 0.055, color);
    painter.line_segment(
        [pos2(c.x, c.y - r * 0.12), pos2(c.x, c.y + r * 0.48)],
        stroke,
    );
}

/// Draws a clean rollback / revert left-return arrow.
pub fn draw_rollback_icon(painter: &Painter, rect: Rect, color: Color32) {
    let s = rect.width().min(rect.height());
    let c = rect.center();
    let stroke = Stroke::new(1.5_f32, color);

    let top_left = pos2(c.x - s * 0.28, c.y - s * 0.12);
    let top_right = pos2(c.x + s * 0.24, c.y - s * 0.12);
    let bot_right = pos2(c.x + s * 0.24, c.y + s * 0.22);
    let bot_left = pos2(c.x - s * 0.12, c.y + s * 0.22);

    painter.line_segment([top_left, top_right], stroke);
    painter.line_segment([top_right, bot_right], stroke);
    painter.line_segment([bot_right, bot_left], stroke);
    // Arrowhead pointing left at top_left
    painter.line_segment(
        [top_left, pos2(top_left.x + s * 0.15, top_left.y - s * 0.14)],
        stroke,
    );
    painter.line_segment(
        [top_left, pos2(top_left.x + s * 0.15, top_left.y + s * 0.14)],
        stroke,
    );
}

/// Draws a clean checkmark.
pub fn draw_check_icon(painter: &Painter, rect: Rect, color: Color32) {
    let s = rect.width().min(rect.height());
    let c = rect.center();
    let stroke = Stroke::new(1.8_f32, color);
    let p1 = pos2(c.x - s * 0.26, c.y + s * 0.02);
    let p2 = pos2(c.x - s * 0.06, c.y + s * 0.22);
    let p3 = pos2(c.x + s * 0.28, c.y - s * 0.20);
    painter.line_segment([p1, p2], stroke);
    painter.line_segment([p2, p3], stroke);
}
