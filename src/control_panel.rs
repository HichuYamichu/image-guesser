use crate::display_window::DisplayWindow;
use crate::viewport::{Gui, GuiImpl};
use crate::{EventLoopState, MyEvent};
use crate::my_image::{create_black_image, DynamicImageConvert};

use egui::epaint::image::ImageDelta;
use egui::epaint::textures::TextureFilter;
use egui::epaint::ColorImage;
use egui::{Context, ImageButton, ImageData, Rect, TextureHandle, Vec2};
use image::io::Reader as ImageReader;
use image::DynamicImage;
use rand::{thread_rng, Rng};
use std::sync::mpsc;
use std::thread;
use winit::window::WindowId;


enum ControlSignal {
    OpenFile(u8, u8),
    Reset(u8, u8),
    RevealTile,
    RevealTileAt(u8, u8),
    Exit,
}

enum Response {
    NewImageLoaded(TextureHandle, TextureHandle),
}

#[derive(PartialEq, Clone, Copy)]
enum Mode {
    Compact,
    Full,
}

pub struct ControlPanel {
    control_tx: mpsc::Sender<ControlSignal>,
    response_rx: mpsc::Receiver<Response>,
    child_window_id: Option<WindowId>,
    full_texture: TextureHandle,
    partial_texture: TextureHandle,

    mode: Mode,
    rows: u8,
    columns: u8,
}

impl ControlPanel {
    pub fn new(ctx: Context) -> Self {
        let (control_tx, control_rx) = mpsc::channel();
        let (response_tx, response_rx) = mpsc::channel();

        let img = create_black_image(1920, 1080);
        let initial_tile_data = gen_tiles(img.width(), img.height(), 4, 4);
        let initial_origial_image = image::DynamicImage::ImageRgba8(img);
        let image_data = ImageData::from(DynamicImageConvert(initial_origial_image.clone()));
        let initial_texture =
            ctx.load_texture("initial_texture", image_data, TextureFilter::Linear);

        spawn_worker_thread(
            ctx,
            control_rx,
            response_tx,
            initial_origial_image,
            initial_texture.clone(),
            initial_tile_data,
        );

        Self {
            control_tx,
            response_rx,
            child_window_id: None,
            full_texture: initial_texture.clone(),
            partial_texture: initial_texture,
            mode: Mode::Compact,
            rows: 4,
            columns: 4,
        }
    }

    pub fn notify_child_ui_has_closed(&mut self, _window_id: WindowId) {
        self.child_window_id = None;
        self.mode = Mode::Compact;
    }
}

impl Gui for ControlPanel {
    fn draw(&mut self, ctx: &egui::Context, state: EventLoopState) {
        if let Ok(res) = self.response_rx.try_recv() {
            match res {
                Response::NewImageLoaded(full_texture, partial_texture) => {
                    self.full_texture = full_texture;
                    self.partial_texture = partial_texture.clone();

                    if let Some(id) = self.child_window_id {
                        let _ = state
                            .event_loop_proxy
                            .send_event(MyEvent::UpdateChildWindowData(id, partial_texture));
                    }
                }
            }
        }

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

                let old_rows = self.rows;
                egui::ComboBox::from_label("Number of rows")
                    .selected_text(format!("{:?}", self.rows))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.rows, 2, "2");
                        ui.selectable_value(&mut self.rows, 4, "4");
                        ui.selectable_value(&mut self.rows, 6, "6");
                        ui.selectable_value(&mut self.rows, 8, "8");
                    });

                let old_columns = self.columns;
                egui::ComboBox::from_label("Number of columns")
                    .selected_text(format!("{:?}", self.columns))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.columns, 2, "2");
                        ui.selectable_value(&mut self.columns, 4, "4");
                        ui.selectable_value(&mut self.columns, 6, "6");
                        ui.selectable_value(&mut self.columns, 8, "8");
                    });

                if old_rows != self.rows || old_columns != self.columns {
                    self.control_tx
                        .send(ControlSignal::Reset(self.rows, self.columns))
                        .expect("Receiver always lives.");
                }

                if ui.button("Reveal").clicked() {
                    self.control_tx
                        .send(ControlSignal::RevealTile)
                        .expect("Receiver always lives");
                }

                ui.radio_value(&mut self.mode, Mode::Compact, "Compact");
                ui.radio_value(&mut self.mode, Mode::Full, "Full");
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.mode == Mode::Compact {
                ui.image(self.partial_texture.id(), ui.available_size());
            } else {
                let space = ui.available_size();
                let space_x = space.x / self.columns as f32;
                let space_y = space.y / self.rows as f32;
                let size: Vec2 = [space_x - 30.0, space_y - 30.0].into();

                egui::Grid::new("buttons").show(ui, |ui| {
                    for x in 0..self.rows {
                        let (start_y, end_y) = (
                            (space_y * x as f32) / space.y,
                            (space_y * (x + 1) as f32) / space.y,
                        );
                        for y in 0..self.columns {
                            let (start_x, end_x) = (
                                (space_x * y as f32) / space.x,
                                (space_x * (y + 1) as f32) / space.x,
                            );

                            let rect = Rect::from_x_y_ranges(start_x..=end_x, start_y..=end_y);
                            let b = ImageButton::new(self.full_texture.id(), size).uv(rect);

                            if ui.add(b).clicked() {
                                self.control_tx
                                    .send(ControlSignal::RevealTileAt(x, y))
                                    .expect("Receiver always lives");
                            }
                        }
                        ui.end_row();
                    }
                });
            }
        });

        match self.child_window_id {
            Some(id) if self.mode == Mode::Compact => {
                // We are currently in Full Mode and wish to switch to Compact
                self.child_window_id = None;
                let _ = state.event_loop_proxy.send_event(MyEvent::CloseWindow(id));
            }
            None if self.mode == Mode::Full => {
                // We are currently in Compact Mode and wish to switch to Full
                let gui = DisplayWindow::new(self.partial_texture.clone());
                let (new_window_id, new_vp) =
                    state.create_window("Image Guesser!", 1920, 1080, GuiImpl::DisplayWindow(gui));
                self.child_window_id = Some(new_window_id);
                let _ = state
                    .event_loop_proxy
                    .send_event(MyEvent::OpenWindow(new_window_id, new_vp));
            }
            _ => {}
        };
    }
}

impl Drop for ControlPanel {
    fn drop(&mut self) {
        let _ = self.control_tx.send(ControlSignal::Exit);
    }
}

fn spawn_worker_thread(
    ctx: Context,
    control_rx: mpsc::Receiver<ControlSignal>,
    response_tx: mpsc::Sender<Response>,
    initial_original_image: DynamicImage,
    initial_texture: TextureHandle,
    intial_tile_data: TileData,
) {
    let mut texture = initial_texture;
    let mut tile_data = intial_tile_data;
    let mut original_image = initial_original_image;

    thread::spawn(move || loop {
        match control_rx.recv().expect("Sender always lives.") {
            ControlSignal::OpenFile(rows, columns) => {
                if let Some(s) = tinyfiledialogs::open_file_dialog("Choose screenshot ;)", "", None)
                {
                    let i = ImageReader::open(s).unwrap().decode().unwrap();
                    let td = gen_tiles(i.width(), i.height(), rows, columns);
                    let original_image_data = ImageData::from(DynamicImageConvert(i.clone()));
                    let full_texture = ctx.load_texture(
                        "full_texture",
                        original_image_data,
                        TextureFilter::Linear,
                    );

                    let black_image =
                        image::DynamicImage::ImageRgba8(create_black_image(i.width(), i.height()));
                    let black_image_data = ImageData::from(DynamicImageConvert(black_image));
                    let partial_texture = ctx.load_texture(
                        "partial_texture",
                        black_image_data,
                        TextureFilter::Linear,
                    );

                    original_image = i;
                    texture = partial_texture;
                    tile_data = td;

                    response_tx
                        .send(Response::NewImageLoaded(full_texture, texture.clone()))
                        .expect("Receiver always lives.");
                };
            }
            ControlSignal::RevealTile => {
                let num_of_tiles = tile_data.tiles.len();
                if num_of_tiles == 0 {
                    continue;
                }

                let tile = tile_data
                    .tiles
                    .remove(thread_rng().gen_range(0..=num_of_tiles - 1));

                let original = &original_image;
                let original_tile = original.crop_imm(
                    tile.x as _,
                    tile.y as _,
                    tile_data.tile_width,
                    tile_data.tile_height,
                );

                let position = [tile.x as _, tile.y as _];
                let delta = ImageDelta::partial(
                    position,
                    ColorImage::from(DynamicImageConvert(original_tile)),
                    TextureFilter::Linear,
                );

                let tex_mgr = ctx.tex_manager();
                tex_mgr.write().set(texture.id(), delta);
            }
            ControlSignal::RevealTileAt(x, y) => {
                let idx = tile_data.tiles.iter().position(|tile| {
                    tile.x as u32 == y as u32 * tile_data.tile_width
                        && tile.y as u32 == x as u32 * tile_data.tile_height
                });

                if let Some(idx) = idx {
                    let tile = tile_data.tiles.remove(idx);
                    let original = &original_image;
                    let original_tile = original.crop_imm(
                        tile.x as _,
                        tile.y as _,
                        tile_data.tile_width,
                        tile_data.tile_height,
                    );

                    let position = [tile.x as _, tile.y as _];
                    let delta = ImageDelta::partial(
                        position,
                        ColorImage::from(DynamicImageConvert(original_tile)),
                        TextureFilter::Linear,
                    );

                    let tex_mgr = ctx.tex_manager();
                    tex_mgr.write().set(texture.id(), delta);
                }
            }
            ControlSignal::Reset(rows, columns) => {
                tile_data = gen_tiles(
                    original_image.width(),
                    original_image.height(),
                    rows,
                    columns,
                );
                let black = image::DynamicImage::ImageRgba8(create_black_image(
                    original_image.width(),
                    original_image.height(),
                ));
                let image_data = ImageData::from(DynamicImageConvert(black));
                let delta = ImageDelta::full(image_data, TextureFilter::Linear);

                tile_data = tile_data;
                let tex_mgr = ctx.tex_manager();
                tex_mgr.write().set(texture.id(), delta);
            }
            ControlSignal::Exit => return,
        }
    });
}

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
