use egui::{Context, FontDefinitions};
use egui_winit_platform::{Platform, PlatformDescriptor};
use enum_dispatch::enum_dispatch;
use winit::window::{Window};

use crate::EventLoopState;
use crate::control_panel::ControlPanel;
use crate::display_window::DisplayWindow;

#[enum_dispatch]
pub trait Gui {
    fn draw(&mut self, ctx: &Context, state: EventLoopState);
}

#[enum_dispatch(Gui)]
pub enum GuiImpl {
    ControlPanel,
    DisplayWindow
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
    pub gui: GuiImpl,
}

impl ViewportDesc {
    pub fn new(window: Window, instance: &wgpu::Instance) -> Self {
        let surface = unsafe { instance.create_surface(&window) };
        Self { window, surface }
    }

    pub fn build<F>(self, adapter: &wgpu::Adapter, device: &wgpu::Device, gui_fn: F) -> Viewport
    where
        F: FnOnce(&egui::Context) -> GuiImpl,
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
