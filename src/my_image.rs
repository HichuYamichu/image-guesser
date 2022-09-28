use egui::ColorImage;
use egui::ImageData;
use image::{DynamicImage, RgbaImage};

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

impl From<DynamicImageConvert> for ImageData {
    fn from(img: DynamicImageConvert) -> Self {
        ImageData::Color(ColorImage::from(img))
    }
}

pub fn create_black_image(width: u32, height: u32) -> RgbaImage {
    let mut empty_image = RgbaImage::new(width, height);
    empty_image.pixels_mut().for_each(|p| p.0 = [0, 0, 0, 255]);
    empty_image
}
