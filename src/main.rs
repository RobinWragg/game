#![allow(unused)]
#![allow(dead_code)]

mod debugger;
mod game;
mod gpu;
mod grid;
mod math;
mod prelude;

use game::Game;
use gpu::ImplGpu;
use prelude::*;
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    monitor::VideoModeHandle,
    window::{Fullscreen, Window, WindowId},
};

const WINDOW_WIDTH: u32 = 1200;
const WINDOW_HEIGHT: u32 = 760;

struct App {
    window: Option<Arc<Window>>,
    gpu: Option<Box<dyn Gpu>>,
    game: Option<Game>,
    mouse_pos: Vec2,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let size = LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT);

        let monitor = event_loop.primary_monitor().unwrap();
        let modes: Vec<VideoModeHandle> = monitor.video_modes().collect();

        // TODO: Choose a sensible video mode for exclusive fullscreen
        let video_mode = modes[0].clone();

        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        // .with_fullscreen(Some(Fullscreen::Exclusive(video_mode)))
                        // .with_fullscreen(Some(Fullscreen::Borderless(None)))
                        .with_inner_size(size)
                        .with_title("game"),
                )
                .unwrap(),
        );

        let impl_gpu = ImplGpu::new(&window);
        self.gpu = Some(Box::new(impl_gpu));
        self.window = Some(window.clone());
        self.game = Some(Game::new());
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.window.as_ref().unwrap().request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let gpu = &mut **self.gpu.as_mut().unwrap();
        let game = self.game.as_mut().unwrap();
        match event {
            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                let size = {
                    let size = self.window.as_ref().unwrap().inner_size();
                    Vec2::new(size.width as f32, size.height as f32)
                };
                self.mouse_pos = Vec2::new(position.x as f32, position.y as f32);
                let normalized_coords = gpu.window_to_normalized(&self.mouse_pos);
                game.push_event(Event::MousePos(normalized_coords));
            }
            WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
            } => {
                if button == MouseButton::Left {
                    match state {
                        ElementState::Pressed => {
                            let normalized_coords = gpu.window_to_normalized(&self.mouse_pos);
                            game.push_event(Event::LeftClickPressed(normalized_coords));
                        }
                        ElementState::Released => {
                            let normalized_coords = gpu.window_to_normalized(&self.mouse_pos);
                            game.push_event(Event::LeftClickReleased(normalized_coords));
                        }
                    }
                } else if button == MouseButton::Right {
                    match state {
                        ElementState::Pressed => {
                            let normalized_coords = gpu.window_to_normalized(&self.mouse_pos);
                            game.push_event(Event::RightClickPressed(normalized_coords));
                        }
                        ElementState::Released => {
                            let normalized_coords = gpu.window_to_normalized(&self.mouse_pos);
                            game.push_event(Event::RightClickReleased(normalized_coords));
                        }
                    }
                }
            }
            WindowEvent::MouseWheel {
                device_id: _,
                delta,
                phase: _,
            } => {
                let scroll_delta = match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => Vec2::new(x, y),
                    winit::event::MouseScrollDelta::PixelDelta(pos) => {
                        Vec2::new(pos.x as f32, pos.y as f32)
                    }
                };
                game.push_event(Event::Scroll(scroll_delta));
            }
            WindowEvent::CloseRequested => event_loop.exit(), // TODO: call this when doing cmd+Q etc
            WindowEvent::RedrawRequested => {
                game.update_and_render(gpu);
            }
            _ => (),
        }
    }
}

fn main() {
    std::env::set_current_dir(
        std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap(),
    )
    .unwrap();

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App {
        game: None,
        window: None,
        gpu: None,
        mouse_pos: Vec2::ZERO,
    };
    let _ = event_loop.run_app(&mut app);
}
