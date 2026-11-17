//! Toasts — notificações temporárias sobrepostas.

use crate::app_state::AppState;
use egui::Context;

pub fn show_toasts(ctx: &Context, state: &mut AppState) {
    if state.toasts.is_empty() {
        return;
    }
    let screen = ctx.screen_rect();
    let mut y = screen.bottom() - 16.0;

    for toast in state.toasts.iter().rev().take(5) {
        let galley = ctx.fonts(|f| {
            f.layout_no_wrap(
                toast.message.clone(),
                egui::FontId::proportional(13.0),
                egui::Color32::WHITE,
            )
        });
        let w = (galley.size().x + 32.0).min(500.0);
        let h = galley.size().y + 16.0;
        y -= h + 8.0;

        let rect =
            egui::Rect::from_min_size(egui::pos2(screen.right() - w - 16.0, y), egui::vec2(w, h));

        ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("toasts"),
        ))
        .rect_filled(rect, 6.0, toast.color().linear_multiply(0.92));

        ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("toasts_text"),
        ))
        .text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            &toast.message,
            egui::FontId::proportional(13.0),
            egui::Color32::WHITE,
        );
    }
}
