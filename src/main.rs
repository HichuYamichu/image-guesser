use std::collections::HashMap;
use std::iter;
use std::time::Instant;

use egui::TextureHandle;
use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use viewport::{Gui, GuiImpl, Viewport, ViewportDesc};
use wgpu::{Adapter, Device, Instance};
use winit::event::{ElementState, Event::*, KeyboardInput, VirtualKeyCode};
use winit::event_loop::{EventLoopProxy, EventLoopWindowTarget};
use winit::window::{Fullscreen, Window, WindowId};

mod control_panel;
mod display_window;
mod my_image;
mod viewport;

const INITIAL_WIDTH: u32 = 1920;
const INITIAL_HEIGHT: u32 = 1080;

pub enum MyEvent {
    OpenWindow(WindowId, Viewport),
    CloseWindow(WindowId),
    UpdateChildWindowData(WindowId, TextureHandle),
}

fn main() {
    let event_loop = winit::event_loop::EventLoopBuilder::<MyEvent>::with_user_event().build();
    let event_loop_proxy = event_loop.create_proxy();
    let instance = wgpu::Instance::new(wgpu::Backends::all());

    let window = create_window(&event_loop, "Image Guesser!", INITIAL_WIDTH, INITIAL_HEIGHT);
    let main_window_id = window.id();
    let main_vp_desc = ViewportDesc::new(window, &instance);

    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        compatible_surface: Some(&main_vp_desc.surface),
        ..Default::default()
    }))
    .expect("Failed to find an appropriate adapter");

    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: None,
            features: wgpu::Features::empty(),
            limits: wgpu::Limits::downlevel_defaults(),
        },
        None,
    ))
    .expect("Failed to create device");

    let vp = main_vp_desc.build(&adapter, &device, |ctx| {
        let main_gui = control_panel::ControlPanel::new(ctx.clone());
        GuiImpl::ControlPanel(main_gui)
    });

    let mut viewports: HashMap<WindowId, Viewport> = HashMap::new();
    viewports.insert(main_window_id, vp);

    let surface_config = viewports.iter().next().unwrap().1.config.format;
    let mut egui_rpass = RenderPass::new(&device, surface_config, 1);

    let start_time = Instant::now();

    event_loop.run(move |event, event_loop, control_flow| {
        let state = EventLoopState {
            event_loop,
            event_loop_proxy: &event_loop_proxy,
            instance: &instance,
            device: &device,
            adapter: &adapter,
        };

        match event {
            RedrawRequested(window_id) => {
                let vp = viewports.get_mut(&window_id).unwrap();
                vp.platform.update_time(start_time.elapsed().as_secs_f64());

                let output_frame = match vp.surface.get_current_texture() {
                    Ok(frame) => frame,
                    Err(wgpu::SurfaceError::Outdated) => {
                        return;
                    }
                    Err(e) => {
                        eprintln!("Dropped frame with error: {}", e);
                        return;
                    }
                };
                let output_view = output_frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                vp.platform.begin_frame();

                vp.gui.draw(&vp.platform.context(), state);

                let full_output = vp.platform.end_frame(Some(&vp.window));
                let paint_jobs = vp.platform.context().tessellate(full_output.shapes);

                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("encoder"),
                });

                let screen_descriptor = ScreenDescriptor {
                    physical_width: vp.config.width,
                    physical_height: vp.config.height,
                    scale_factor: vp.window.scale_factor() as f32,
                };
                let tdelta: egui::TexturesDelta = full_output.textures_delta;
                egui_rpass
                    .add_textures(&device, &queue, &tdelta)
                    .expect("add texture ok");
                egui_rpass.update_buffers(&device, &queue, &paint_jobs, &screen_descriptor);

                egui_rpass
                    .execute(
                        &mut encoder,
                        &output_view,
                        &paint_jobs,
                        &screen_descriptor,
                        Some(wgpu::Color::BLACK),
                    )
                    .unwrap();
                queue.submit(iter::once(encoder.finish()));

                output_frame.present();

                egui_rpass
                    .remove_textures(tdelta)
                    .expect("remove texture ok");
            }
            UserEvent(e) => match e {
                MyEvent::OpenWindow(window_id, viewport) => {
                    viewports.insert(window_id, viewport);
                }
                MyEvent::CloseWindow(window_id) => {
                    viewports.remove(&window_id);
                }
                MyEvent::UpdateChildWindowData(window_id, texture) => {
                    let vp = viewports
                        .get_mut(&window_id)
                        .expect("This id must be present in map.");
                    match &mut vp.gui {
                        GuiImpl::ControlPanel(_) => {
                            unreachable!("ControlPanel is never a child window.")
                        }
                        GuiImpl::DisplayWindow(ref mut dp) => dp.update_texture(texture),
                    }
                }
            },
            MainEventsCleared => {
                viewports.iter().for_each(|(_, vp)| {
                    vp.window.request_redraw();
                });
            }
            WindowEvent {
                event: ref window_event,
                window_id,
            } => {
                match viewports.get_mut(&window_id) {
                    Some(vp) => vp.platform.handle_event(&event),
                    None => {}
                };

                match window_event {
                    winit::event::WindowEvent::Resized(size) => {
                        if let Some(vp) = viewports.get_mut(&window_id) {
                            // Resize with 0 width and height is used by winit to signal a minimize event on Windows.
                            // See: https://github.com/rust-windowing/winit/issues/208
                            // This solves an issue where the app would panic when minimizing on Windows.
                            if size.width > 0 && size.height > 0 {
                                vp.config.width = size.width;
                                vp.config.height = size.height;
                                vp.surface.configure(&device, &vp.config);
                            }
                        }
                    }
                    winit::event::WindowEvent::CloseRequested => {
                        if window_id == main_window_id {
                            viewports.drain();
                            control_flow.set_exit();
                        } else {
                            let gui = &mut viewports.get_mut(&main_window_id).unwrap().gui;
                            match gui {
                                GuiImpl::ControlPanel(cp) => {
                                    cp.notify_child_ui_has_closed(window_id);
                                }
                                GuiImpl::DisplayWindow(_) => unreachable!(),
                            }

                            viewports.remove(&window_id);
                            if viewports.is_empty() {
                                control_flow.set_exit();
                            }
                        }
                    }

                    winit::event::WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                virtual_keycode: Some(virtual_code),
                                state: ElementState::Pressed,
                                ..
                            },
                        ..
                    } => {
                        if let Some(vp) = viewports.get_mut(&window_id) {
                            match virtual_code {
                                VirtualKeyCode::Escape if vp.window.fullscreen().is_some() => {
                                    vp.window.set_fullscreen(None);
                                }
                                VirtualKeyCode::F11 if vp.window.fullscreen().is_some() => {
                                    vp.window.set_fullscreen(None);
                                }
                                VirtualKeyCode::F11 if vp.window.fullscreen().is_none() => {
                                    let fullscreen = Some(Fullscreen::Borderless(None));
                                    vp.window.set_fullscreen(fullscreen);
                                }
                                _ => (),
                            }
                        }
                    }
                    _ => {}
                }
            }
            _ => (),
        }
    });
}

pub struct EventLoopState<'a> {
    event_loop: &'a EventLoopWindowTarget<MyEvent>,
    event_loop_proxy: &'a EventLoopProxy<MyEvent>,
    adapter: &'a Adapter,
    instance: &'a Instance,
    device: &'a Device,
}

impl<'a> EventLoopState<'a> {
    pub fn create_window<T>(
        &self,
        title: T,
        width: u32,
        height: u32,
        gui: GuiImpl,
    ) -> (WindowId, Viewport)
    where
        T: Into<String>,
    {
        let window = create_window(self.event_loop, title, width, height);
        let vp_desc = ViewportDesc::new(window, self.instance);
        let vp = vp_desc.build(self.adapter, self.device, |_| gui);
        let window_id = vp.window.id();

        (window_id, vp)
    }
}

fn create_window<T>(
    event_loop: &EventLoopWindowTarget<MyEvent>,
    title: T,
    width: u32,
    height: u32,
) -> Window
where
    T: Into<String>,
{
    winit::window::WindowBuilder::new()
        .with_decorations(true)
        .with_resizable(true)
        .with_transparent(false)
        .with_maximized(true)
        .with_title(title)
        .with_inner_size(winit::dpi::PhysicalSize { width, height })
        .build(event_loop)
        .unwrap()
}
