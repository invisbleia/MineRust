//! MineRust spike (throwaway): proves the game's own stack — wgpu 30 WebGPU +
//! winit 0.30 — draws into a browser tab's canvas. No gameplay, no shims.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use wasm_bindgen::prelude::*;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
}

#[derive(Default)]
struct State {
    gpu: Option<Gpu>,
}

struct App {
    state: Rc<RefCell<State>>,
    window: Option<Arc<Window>>,
}

impl App {
    fn boot(&mut self, el: &ActiveEventLoop) {
        let canvas = web_sys::window()
            .expect("js window")
            .document()
            .expect("document")
            .get_element_by_id("game")
            .expect("#game canvas")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("a canvas element");
        let attrs = Window::default_attributes()
            .with_title("MineRust spike")
            .with_canvas(Some(canvas));
        let window = Arc::new(el.create_window(attrs).expect("window"));
        self.window = Some(window.clone());

        let state = self.state.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let instance = wgpu::Instance::default();
            let surface = instance
                .create_surface(window.clone())
                .expect("surface from canvas");
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    surface: Some(&surface),
                    ..Default::default()
                })
                .await
                .expect("WebGPU adapter — is this browser WebGPU-capable?");
            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor {
                    label: Some("MineRust spike"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    ..Default::default()
                })
                .await
                .expect("device");
            let size = window.inner_size();
            let config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: surface
                    .get_capabilities(&adapter)
                    .formats
                    .first()
                    .copied()
                    .expect("surface formats"),
                width: size.width.max(1),
                height: size.height.max(1),
                present_mode: wgpu::PresentMode::AutoVsync,
                alpha_mode: wgpu::CompositeAlphaMode::Auto,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            };
            surface.configure(&device, &config);
            state.borrow_mut().gpu = Some(Gpu {
                device,
                queue,
                surface,
                config,
            });
            web_sys::console::log_1(&"MineRust spike: gpu ready, drawing".into());
            window.request_redraw();
        });
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        self.boot(el);
    }

    fn window_event(&mut self, _el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if let WindowEvent::RedrawRequested = event {
            let state = self.state.borrow();
            let Some(gpu) = state.gpu.as_ref() else {
                return;
            };
            let Ok(frame) = gpu.surface.get_current_texture() else {
                return;
            };
            let view = frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let mut encoder = gpu
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("spike clear"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.09,
                                g: 0.45,
                                b: 0.09,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                pass.clear_color(wgpu::Color {
                    r: 0.09,
                    g: 0.45,
                    b: 0.09,
                    a: 1.0,
                });
            }
            gpu.queue.submit([encoder.finish()]);
            frame.present();
        }
    }

    fn about_to_wait(&mut self, _el: &ActiveEventLoop) {
        // Redraw every frame once the GPU is up: the spike's "game loop".
        if self.state.borrow().gpu.is_some() {
            if let Some(window) = self.window.as_ref() {
                window.request_redraw();
            }
        }
    }
}

#[wasm_bindgen(start)]
pub async fn run() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    let app = App {
        state: Rc::new(RefCell::new(State::default())),
        window: None,
    };
    let event_loop = EventLoop::new().map_err(|e| JsValue::from_str(&e.to_string()))?;
    event_loop
        .run_app(app)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}
