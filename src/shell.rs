use crate::theme::{self, fg_a, px, text, bg, fg};
use eframe::egui::{self, Align2, Color32, Pos2, Rect, Sense, Shape, Stroke, Vec2};

/// Cornice del terminale con angoli tagliati (bl e tr) come l'augmented-ui originale.
pub fn frame(p: &egui::Painter, r: Rect, vh: f32) {
    let c = vh * 2.2;
    let pts = vec![
        r.left_top(),
        Pos2::new(r.right() - c, r.top()),
        Pos2::new(r.right(), r.top() + c),
        r.right_bottom(),
        Pos2::new(r.left() + c, r.bottom()),
        Pos2::new(r.left(), r.bottom() - c),
        r.left_top(),
    ];
    p.add(Shape::line(pts, Stroke::new(px(vh, 0.18), fg_a(0.5))));
}

/// Barra delle tab inclinate (skewX 35deg). Ritorna l'indice cliccato.
pub fn tabs(ui: &mut egui::Ui, r: Rect, labels: &[String], active: usize, vh: f32) -> Option<usize> {
    let p = ui.painter_at(r.expand2(Vec2::new(0.0, vh)));
    let n = labels.len();
    let w = r.width() / n as f32;
    let h = r.height();
    let skew = (35.0f32).to_radians().tan() * h / 2.0;
    let mut clicked = None;

    let draw = |i: usize, scale: f32| {
        let cx = r.left() + w * (i as f32 + 0.5);
        let (hw, hh) = (w * 0.5 * scale, h * 0.5 * scale);
        let cy = r.center().y;
        let s = skew * scale;
        let poly = vec![
            Pos2::new(cx - hw - s, cy - hh),
            Pos2::new(cx + hw - s, cy - hh),
            Pos2::new(cx + hw + s, cy + hh),
            Pos2::new(cx - hw + s, cy + hh),
        ];
        let is_active = i == active;
        let fill = if is_active { fg() } else { bg() };
        p.add(Shape::convex_polygon(poly.clone(), fill, Stroke::new(px(vh, 0.18), fg_a(0.5))));
        let col = if is_active { bg() } else { fg() };
        let fam = "main";
        text(&p, cx, cy + vh * 0.15, Align2::CENTER_CENTER, &labels[i], vh * 1.7 * scale, col, fam);
    };
    for i in 0..n {
        if i != active {
            draw(i, 1.0);
        }
    }
    draw(active, 1.12);

    for i in 0..n {
        let tr = Rect::from_min_size(Pos2::new(r.left() + w * i as f32, r.top()), Vec2::new(w, h));
        let resp = ui.interact(tr, ui.id().with(("tab", i)), Sense::click());
        if resp.clicked() {
            clicked = Some(i);
        }
    }
    let line = r.bottom();
    p.line_segment(
        [Pos2::new(r.left(), line), Pos2::new(r.right(), line)],
        Stroke::new(px(vh, 0.18), fg_a(0.5)),
    );
    let _ = Color32::BLACK;
    let _ = theme::fg();
    clicked
}
