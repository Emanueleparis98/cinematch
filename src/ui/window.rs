use crate::ui::lumen;
const RESIZE_EDGE: f32 = 6.0;
pub(crate) enum CaptionIcon {
    Minimize,
    Maximize,
    Restore,
    Close,
}

fn resize_edge(rect: egui::Rect, pos: egui::Pos2) -> Option<egui::viewport::ResizeDirection> {
    use egui::viewport::ResizeDirection as Dir;
    let left = pos.x <= rect.left() + RESIZE_EDGE;
    let right = pos.x >= rect.right() - RESIZE_EDGE;
    let top = pos.y <= rect.top() + RESIZE_EDGE;
    let bottom = pos.y >= rect.bottom() - RESIZE_EDGE;
    match (left, right, top, bottom) {
        (true, _, true, _) => Some(Dir::NorthWest),
        (_, true, true, _) => Some(Dir::NorthEast),
        (true, _, _, true) => Some(Dir::SouthWest),
        (_, true, _, true) => Some(Dir::SouthEast),
        (true, _, _, _) => Some(Dir::West),
        (_, true, _, _) => Some(Dir::East),
        (_, _, true, _) => Some(Dir::North),
        (_, _, _, true) => Some(Dir::South),
        _ => None,
    }
}

pub(crate) fn window_resize_edges(ctx: &egui::Context) {
    if ctx.input(|i| i.viewport().maximized.unwrap_or(false)) {
        return;
    }
    let Some(pos) = ctx.pointer_hover_pos() else {
        return;
    };
    let Some(dir) = resize_edge(ctx.viewport_rect(), pos) else {
        return;
    };
    use egui::viewport::ResizeDirection as Dir;
    ctx.set_cursor_icon(match dir {
        Dir::North | Dir::South => egui::CursorIcon::ResizeVertical,
        Dir::East | Dir::West => egui::CursorIcon::ResizeHorizontal,
        Dir::NorthWest | Dir::SouthEast => egui::CursorIcon::ResizeNwSe,
        Dir::NorthEast | Dir::SouthWest => egui::CursorIcon::ResizeNeSw,
    });
    if ctx.input(|i| i.pointer.primary_pressed()) {
        ctx.send_viewport_cmd(egui::viewport::ViewportCommand::BeginResize(dir));
    }
}

pub(crate) fn window_caption(
    ui: &mut egui::Ui,
    icon: CaptionIcon,
    tip: &str,
    cap: f32,
) -> egui::Response {
    let resp = ui.add(
        egui::Button::new("")
            .frame(false)
            .min_size(egui::vec2(cap, 40.0)),
    );
    let hover = ui
        .ctx()
        .animate_bool_with_time(resp.id, resp.hovered(), 0.12);
    if hover > 0.01 {
        let fill = if matches!(icon, CaptionIcon::Close) {
            lumen::mix(lumen::SURFACE, lumen::DANGER, hover)
        } else {
            lumen::mix(lumen::INK, lumen::RAISED, hover)
        };
        ui.painter()
            .rect_filled(resp.rect, egui::CornerRadius::ZERO, fill);
    }
    let color = if matches!(icon, CaptionIcon::Close) {
        lumen::mix(lumen::MUTED, egui::Color32::WHITE, hover)
    } else {
        lumen::mix(lumen::MUTED, lumen::IVORY, hover)
    };
    let stroke = egui::Stroke::new(1.4_f32, color);
    let c = resp.rect.center();
    let p = ui.painter();
    match icon {
        CaptionIcon::Minimize => {
            p.line_segment(
                [c + egui::vec2(-4.2, 0.5), c + egui::vec2(4.2, 0.5)],
                stroke,
            );
        }
        CaptionIcon::Maximize => {
            p.rect_stroke(
                egui::Rect::from_center_size(c, egui::vec2(8.5, 8.5)),
                lumen::radius(0.5),
                stroke,
                egui::StrokeKind::Inside,
            );
        }
        CaptionIcon::Restore => {
            let back =
                egui::Rect::from_center_size(c + egui::vec2(1.5, -1.5), egui::vec2(7.0, 7.0));
            let front =
                egui::Rect::from_center_size(c + egui::vec2(-1.5, 1.5), egui::vec2(7.0, 7.0));
            // Del riquadro dietro restano visibili solo alto e lato destro.
            p.line_segment([back.left_top(), back.right_top()], stroke);
            p.line_segment([back.right_top(), back.right_bottom()], stroke);
            p.rect_stroke(front, lumen::radius(0.5), stroke, egui::StrokeKind::Inside);
        }
        CaptionIcon::Close => {
            let d = 4.2;
            p.line_segment([c + egui::vec2(-d, -d), c + egui::vec2(d, d)], stroke);
            p.line_segment([c + egui::vec2(d, -d), c + egui::vec2(-d, d)], stroke);
        }
    }
    lumen::focus(ui, &resp, 4.0);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(tip)
}
