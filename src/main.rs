// TODO: Remove.
#![allow(unused)]
#![allow(dead_code)]

mod debugger;
mod game;
mod gpu;
mod grid;
mod prelude;

use game::Game;
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
const WINDOW_HEIGHT: u32 = 675;

struct App<'a> {
    window: Option<Arc<Window>>,
    gpu: Option<Gpu<'a>>,
    game: Option<Game>,
    mouse_pos: Vec2,
}

impl ApplicationHandler for App<'_> {
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

        self.gpu = Some(Gpu::new(&window));
        self.window = Some(window.clone());
        self.game = Some(Game::new());
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.window.as_ref().unwrap().request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let gpu = self.gpu.as_mut().unwrap();
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

                // Convert to NDC
                self.mouse_pos /= size * 0.5;
                self.mouse_pos -= 1.0;
                self.mouse_pos.y *= -1.0;
                game.push_event(Event::MousePos(self.mouse_pos));
            }
            WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
            } => {
                if button == MouseButton::Left {
                    match state {
                        ElementState::Pressed => {
                            game.push_event(Event::LeftClickPressed(self.mouse_pos));
                        }
                        ElementState::Released => {
                            game.push_event(Event::LeftClickReleased(self.mouse_pos));
                        }
                    }
                }
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
