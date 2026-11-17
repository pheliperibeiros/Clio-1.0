//! Componentes e widgets reutilizáveis para a interface do Clio.
#![forbid(unsafe_code)]

use egui::{Color32, Rect, Response, Sense, Stroke, Ui, Vec2, Widget};

/// Widget para exibição gráfica de medidor VU estéreo/mono.
pub struct VuMeterWidget {
    rms_normalized: f32,
    peak_normalized: f32,
    clipping: bool,
    width: f32,
    height: f32,
}

impl VuMeterWidget {
    pub fn new(rms_normalized: f32, peak_normalized: f32, clipping: bool) -> Self {
        Self {
            rms_normalized: rms_normalized.clamp(0.0, 1.0),
            peak_normalized: peak_normalized.clamp(0.0, 1.0),
            clipping,
            width: 160.0,
            height: 12.0,
        }
    }

    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }
}

impl Widget for VuMeterWidget {
    fn ui(self, ui: &mut Ui) -> Response {
        let desired_size = Vec2::new(self.width, self.height);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::hover());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();

            // Fundo escuro do medidor
            painter.rect_filled(rect, 2.0, Color32::from_rgb(25, 25, 30));

            // Preenchimento RMS por faixas de volume
            let fill_width = self.width * self.rms_normalized;
            if fill_width > 0.0 {
                let fill_rect = Rect::from_min_size(rect.min, Vec2::new(fill_width, self.height));
                let color = if self.clipping {
                    Color32::from_rgb(230, 45, 45) // Vermelho clipping
                } else if self.rms_normalized > 0.82 {
                    Color32::from_rgb(230, 190, 40) // Amarelo alerta
                } else {
                    Color32::from_rgb(45, 180, 80) // Verde normal
                };
                painter.rect_filled(fill_rect, 2.0, color);
            }

            // Marcador de Pico (linha vertical sutil)
            if self.peak_normalized > 0.02 {
                let peak_x = rect.min.x + (self.width * self.peak_normalized).min(self.width - 2.0);
                painter.line_segment(
                    [
                        egui::pos2(peak_x, rect.min.y),
                        egui::pos2(peak_x, rect.max.y),
                    ],
                    Stroke::new(1.5_f32, Color32::WHITE),
                );
            }

            // Borda externa
            painter.rect_stroke(
                rect,
                2.0,
                Stroke::new(1.0_f32, Color32::from_gray(60)),
                egui::StrokeKind::Inside,
            );
        }

        response
    }
}

/// Indicador circular de status de gravação pulsante.
pub struct RecordingIndicator {
    pub is_recording: bool,
    pub is_paused: bool,
}

impl Widget for RecordingIndicator {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = Vec2::new(18.0, 18.0);
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let center = rect.center();

            if self.is_recording {
                let color = if self.is_paused {
                    Color32::from_rgb(230, 180, 20)
                } else {
                    Color32::from_rgb(225, 40, 40)
                };
                painter.circle_filled(center, 7.0, color);
            } else {
                painter.circle_stroke(center, 6.0, Stroke::new(1.5_f32, Color32::from_gray(100)));
            }
        }

        response
    }
}
