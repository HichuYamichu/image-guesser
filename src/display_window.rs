use egui::{style::Margin, Frame};
use winit::window::WindowId;

use crate::{my_image::MyImage, viewport::Gui, EventLoopState};

pub struct DisplayWindow {
    image: MyImage,
    frame: Frame,
}

impl DisplayWindow {
    pub fn new(image: MyImage) -> Self {
        let frame = Frame {
            inner_margin: Margin::same(0.0),
            ..Default::default()
        };

        Self { image, frame  }
    }
}

impl Gui for DisplayWindow {
    fn draw(&mut self, ctx: &egui::Context, _state: EventLoopState) {
        egui::CentralPanel::default().frame(self.frame).show(ctx, |ui| {
            self.image.ui(ui);
        });
    }

    fn notify_child_ui_has_closed(&mut self, _window_id: WindowId) {
    }
}
