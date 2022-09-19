use crate::display_window::DisplayWindow;
use crate::viewport::Gui;
use crate::{EventLoopState, MyEvent};
use egui::{ImageButton, Vec2, Rect};
use egui::epaint::image::ImageDelta;
use egui::epaint::textures::TextureFilter;
use egui::epaint::ColorImage;
use image::io::Reader as ImageReader;
use image::DynamicImage;
use rand::{thread_rng, Rng};
use std::ops::Index;
use std::sync::mpsc;
use std::thread;
use winit::window::WindowId;

use crate::my_image::{create_black_image, DynamicImageConvert, MyImage};

enum ControlSignal {
    OpenFile(u8, u8),
    SetNumOfSquares(u8, u8),
    Reset(u8, u8),
    RevealSquare,
    RevealSquareAt(u8, u8),
    Exit,
}

#[derive(PartialEq, Clone, Copy)]
enum Mode {
    Compact,
    Full,
}

pub struct ControlPanel {
    control_tx: mpsc::Sender<ControlSignal>,
    child_window_id: Option<WindowId>,
    image: MyImage,
    mode: Mode,

    rows: u8,
    columns: u8,
}

impl ControlPanel {
    pub fn new(image: MyImage) -> Self {
        let (control_tx, control_rx) = mpsc::channel();
        spawn_worker_thread(image.clone(), control_rx);

        Self {
            control_tx,
            child_window_id: None,
            image,
            mode: Mode::Compact,
            rows: 4,
            columns: 4,
        }
    }
}

impl Gui for ControlPanel {
    fn draw(&mut self, ctx: &egui::Context, state: EventLoopState) {
        // ctx.set_debug_on_hover(true);
        let mut s = (*ctx.style()).clone();
        s.spacing.button_padding = [10.0, 10.0].into();
        ctx.set_style(s);

        egui::TopBottomPanel::top("Control panel").show(ctx, |ui| {
            ui.set_height(60.0);
            ui.horizontal_centered(|ui| {
                ui.heading("Image Guesser!");

                if ui.button("Open file").clicked() {
                    self.control_tx
                        .send(ControlSignal::OpenFile(self.rows, self.columns))
                        .expect("Receiver always lives");
                }
                if ui.button("Reset").clicked() {
                    self.control_tx
                        .send(ControlSignal::Reset(self.rows, self.columns))
                        .expect("Receiver always lives");
                }

                egui::ComboBox::from_label("Number of rows")
                    .selected_text(format!("{:?}", self.rows))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.rows, 2, "2");
                        ui.selectable_value(&mut self.rows, 4, "4");
                        ui.selectable_value(&mut self.rows, 6, "6");
                        ui.selectable_value(&mut self.rows, 8, "8");
                    });

                egui::ComboBox::from_label("Number of columns")
                    .selected_text(format!("{:?}", self.columns))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.columns, 2, "2");
                        ui.selectable_value(&mut self.columns, 4, "4");
                        ui.selectable_value(&mut self.columns, 6, "6");
                        ui.selectable_value(&mut self.columns, 8, "8");
                    });

                if ui.button("Reveal").clicked() {
                    self.control_tx
                        .send(ControlSignal::RevealSquare)
                        .expect("Receiver always lives");
                }

                ui.radio_value(&mut self.mode, Mode::Compact, "Compact");
                ui.radio_value(&mut self.mode, Mode::Full, "Full");
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.mode == Mode::Compact {
                self.image.ui(ui);
            } else {
                let size: Vec2 = [100.0, 100.0].into();
                let rect = Rect::from_x_y_ranges(0.0..=0.4, 0.0..=0.4);
                let b = ImageButton::new(self.image.tex.id(), size).uv(rect);
                ui.add(b);
                
                egui::Grid::new("buttons").show(ui, |ui| {
                    ui.style_mut().spacing.button_padding = [100.0, 100.0].into();
                    for x in 0..self.rows {
                        for y in 0..self.columns {
                            if ui.button("aa").clicked() {
                                self.control_tx
                                    .send(ControlSignal::RevealSquareAt(x, y))
                                    .expect("Receiver always lives");
                            }
                        }
                        ui.end_row();
                    }
                });
            }
        });

        match self.child_window_id {
            Some(id) => {
                if self.mode == Mode::Compact {
                    self.child_window_id = None;
                    let _ = state.event_loop_proxy.send_event(MyEvent::CloseWindow(id));
                }
            }
            None => {
                if self.mode == Mode::Full {
                    let gui = Box::new(DisplayWindow::new(self.image.clone()));
                    let (new_window_id, new_vp) =
                        state.create_window("Image Guesser!", 1920, 1080, gui);
                    self.child_window_id = Some(new_window_id);
                    let _ = state
                        .event_loop_proxy
                        .send_event(MyEvent::OpenWindow(new_window_id, new_vp));
                }
            }
        }
    }

    fn notify_child_ui_has_closed(&mut self, _window_id: WindowId) {
        self.child_window_id = None;
        self.mode = Mode::Compact;
    }
}

impl Drop for ControlPanel {
    fn drop(&mut self) {
        let _ = self.control_tx.send(ControlSignal::Exit);
    }
}

fn spawn_worker_thread(image: MyImage, control_rx: mpsc::Receiver<ControlSignal>) {
    thread::spawn(move || {
        let mut thread_data = None;
        loop {
            match control_rx.recv().expect("Sender always lives") {
                ControlSignal::OpenFile(rows, columns) => {
                    if let Some(s) =
                        tinyfiledialogs::open_file_dialog("Choose screenshot ;)", "", None)
                    {
                        let i = ImageReader::open(s).unwrap().decode().unwrap();
                        let black = create_black_image(i.width(), i.height());
                        let black = image::DynamicImage::ImageRgba8(black);
                        let tile_data = gen_tiles(i.width(), i.height(), rows, columns);

                        thread_data = Some(ThreadData {
                            tile_data,
                            original_image: i,
                        });

                        let delta = ImageDelta::full(
                            ColorImage::from(DynamicImageConvert(black)),
                            TextureFilter::Linear,
                        );
                        image.update(delta)
                    };
                }
                ControlSignal::RevealSquare => {
                    if let Some(ref mut data) = thread_data {
                        if data.tile_data.tiles.is_empty() {
                            continue;
                        }

                        let tile = data
                            .tile_data
                            .tiles
                            .remove(thread_rng().gen_range(0..=data.tile_data.tiles.len() - 1));

                        let original = &data.original_image;
                        let original_tile = original.crop_imm(
                            tile.x as _,
                            tile.y as _,
                            data.tile_data.tile_width,
                            data.tile_data.tile_height,
                        );

                        let position = [tile.x as _, tile.y as _];
                        let delta = ImageDelta::partial(
                            position,
                            ColorImage::from(DynamicImageConvert(original_tile)),
                            TextureFilter::Linear,
                        );
                        image.update(delta);
                    }
                }
                ControlSignal::RevealSquareAt(x, y) => {
                    if let Some(ref mut data) = thread_data {
                        if data.tile_data.tiles.is_empty() {
                            continue;
                        }

                        let idx = data.tile_data.tiles.iter().position(|tile| {
                            tile.x as u32 == y as u32 * data.tile_data.tile_width
                                && tile.y as u32 == x as u32 * data.tile_data.tile_height
                        });

                        let idx = if let Some(idx) = idx {
                            idx
                        } else {
                            continue;
                        };

                        let tile = data.tile_data.tiles.remove(idx);

                        let original = &data.original_image;
                        let original_tile = original.crop_imm(
                            tile.x as _,
                            tile.y as _,
                            data.tile_data.tile_width,
                            data.tile_data.tile_height,
                        );

                        let position = [tile.x as _, tile.y as _];
                        let delta = ImageDelta::partial(
                            position,
                            ColorImage::from(DynamicImageConvert(original_tile)),
                            TextureFilter::Linear,
                        );
                        image.update(delta);
                    }
                }
                ControlSignal::SetNumOfSquares(rows, columns) => {
                    if let Some(ref mut data) = thread_data {
                        let tile_data = gen_tiles(
                            data.original_image.width(),
                            data.original_image.height(),
                            rows,
                            columns,
                        );
                        data.tile_data = tile_data;
                    }
                }
                ControlSignal::Reset(rows, columns) => {
                    if let Some(ref mut data) = thread_data {
                        let tile_data = gen_tiles(
                            data.original_image.width(),
                            data.original_image.height(),
                            rows,
                            columns,
                        );
                        data.tile_data = tile_data;

                        let black = create_black_image(
                            data.original_image.width(),
                            data.original_image.height(),
                        );
                        let black = image::DynamicImage::ImageRgba8(black);
                        let delta = ImageDelta::full(
                            ColorImage::from(DynamicImageConvert(black)),
                            TextureFilter::Linear,
                        );
                        image.update(delta)
                    }
                }
                ControlSignal::Exit => return,
            }
        }
    });

    #[derive(Copy, Clone, Debug)]
    struct Tile {
        x: i64,
        y: i64,
    }

    #[derive(Clone, Debug)]
    struct TileData {
        tiles: Vec<Tile>,
        tile_width: u32,
        tile_height: u32,
    }

    struct ThreadData {
        tile_data: TileData,
        original_image: DynamicImage,
    }

    fn gen_tiles(width: u32, height: u32, rows: u8, columns: u8) -> TileData {
        let tile_height = height / rows as u32;
        let tile_width = width / columns as u32;

        let mut tiles = vec![];
        for x in 0..rows {
            let pos_y = x as u32 * tile_height;
            for y in 0..columns {
                let pos_x = y as u32 * tile_width;
                tiles.push(Tile {
                    x: pos_x as _,
                    y: pos_y as _,
                });
            }
        }

        TileData {
            tiles,
            tile_width,
            tile_height,
        }
    }
}
