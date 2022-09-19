use egui::epaint::ImageDelta;
use egui::mutex::RwLock;
use egui::{Context, TextureId};
use egui::{epaint::TextureManager, ColorImage, TextureFilter, TextureHandle};
use image::{DynamicImage, RgbaImage};
use std::sync::Arc;

pub struct DynamicImageConvert(pub DynamicImage);

impl From<DynamicImageConvert> for ColorImage {
    fn from(img: DynamicImageConvert) -> Self {
        let img = img.0;
        let size = [img.width() as _, img.height() as _];
        let image_buffer = img.to_rgba8();
        let pixels = image_buffer.as_flat_samples();
        egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice())
    }
}

#[derive(Clone)]
pub struct MyImage {
    ctx: Context,
    pub tex: TextureHandle,
}
// TODO: rework this again
impl MyImage {
    pub fn new(ctx: &egui::Context) -> Self {
        let img = create_black_image(1920, 1080);
        let img = image::DynamicImage::ImageRgba8(img);
        let tex = ctx.load_texture(
            "MyImageTexture",
            ColorImage::from(DynamicImageConvert(img)),
            TextureFilter::Linear,
        );

        Self {
            ctx: ctx.clone(),
            tex,
        }
    }

    pub fn load(&self, img: image::DynamicImage) {
        let tex = self.ctx.load_texture(
            "MyImageTexture",
            ColorImage::from(DynamicImageConvert(img)),
            TextureFilter::Linear,
        );

    }

    pub fn update(&self, delta: ImageDelta) {
        let tex_mgr = self.ctx.tex_manager();
        tex_mgr.write().set(self.tex.id(), delta);
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.image(self.tex.id(), ui.available_size());
    }
}

pub fn create_black_image(tile_width: u32, tile_height: u32) -> RgbaImage {
    let mut empty_tile = RgbaImage::new(tile_width, tile_height);
    empty_tile.pixels_mut().for_each(|p| p.0 = [0, 0, 0, 255]);
    empty_tile
}

struct Imaginator {
    inner: Arc<RwLock<Inner>>,
    ctx: Context,
}

impl Imaginator {
    pub fn new(ctx: &egui::Context) -> Self {
    }

    pub fn load(&self, image: DynamicImage) {
        let tile_data = gen_tiles(image.width(), image.height(), rows, columns);

    }

    pub fn update(&self, image: DynamicImage) {
        let tile_data = gen_tiles(image.width(), image.height(), rows, columns);

    }

    pub fn get_full_texture() -> TextureId {}
    pub fn get_partial_texture() -> TextureId {}
}

struct Inner {
    original_image: DynamicImage,
    full_texture: TextureHandle,
    partial_texture: TextureHandle,
    tile_data: TileData,
    rows: u8,
    colums: u8,
}