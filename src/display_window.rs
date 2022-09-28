use egui::{style::Margin, Frame, TextureHandle};

use crate::{viewport::Gui, EventLoopState};

pub struct DisplayWindow {
    texture: TextureHandle,
    frame: Frame,
}

impl DisplayWindow {
    pub fn new(texture: TextureHandle) -> Self {
        let frame = Frame {
            inner_margin: Margin::same(0.0),
            ..Default::default()
        };

        Self { texture, frame }
    }
}

impl DisplayWindow {
    pub fn update_texture(&mut self, tex: TextureHandle) {
        self.texture = tex;
    }
}

impl Gui for DisplayWindow {
    fn draw(&mut self, ctx: &egui::Context, _state: EventLoopState) {
        egui::CentralPanel::default()
            .frame(self.frame)
            .show(ctx, |ui| {
                ui.image(self.texture.id(), ui.available_size());
            });
    }
}
