use egui::{Context, FontDefinitions};
use egui_winit_platform::{Platform, PlatformDescriptor};
use winit::window::{Window, WindowId};

use crate::EventLoopState;

pub trait Gui {
    fn draw(&mut self, ctx: &Context, state: EventLoopState);
    fn notify_child_ui_has_closed(&mut self, window_id: WindowId);
}

pub struct ViewportDesc {
    pub window: Window,
    pub surface: wgpu::Surface,
}

pub struct Viewport {
    pub config: wgpu::SurfaceConfiguration,
    pub platform: Platform,
    pub window: Window,
    pub surface: wgpu::Surface,
    pub gui: Box<dyn Gui>,
}

impl ViewportDesc {
    pub fn new(window: Window, instance: &wgpu::Instance) -> Self {
        let surface = unsafe { instance.create_surface(&window) };
        Self { window, surface }
    }

    pub fn build<F>(self, adapter: &wgpu::Adapter, device: &wgpu::Device, gui_fn: F) -> Viewport
    where
        F: FnOnce(&egui::Context) -> Box<dyn Gui>,
    {
        let size = self.window.inner_size();

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface.get_supported_formats(adapter)[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };

        self.surface.configure(device, &config);

        let platform = Platform::new(PlatformDescriptor {
            physical_width: size.width as u32,
            physical_height: size.height as u32,
            scale_factor: self.window.scale_factor(),
            font_definitions: FontDefinitions::default(),
            style: Default::default(),
        });

        let gui = gui_fn(&platform.context());

        Viewport {
            window: self.window,
            surface: self.surface,
            config,
            platform,
            gui,
        }
    }
}
