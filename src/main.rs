mod game;
mod render;
mod terrain;
mod viewpoint;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use game::Game;
use render::Renderer;
use viewpoint::Viewpoint;
use winit::dpi::LogicalSize;
use winit::event::{DeviceEvent, ElementState, Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Fullscreen, Window};

fn main() {
    if let Err(error) = run() {
        eprintln!("BlockWorld: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut windowed = false;
    let mut profile = false;
    let mut seed = None;
    let mut viewpoint = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--windowed" => windowed = true,
            "--profile" => profile = true,
            "--seed" => {
                seed = Some(
                    args.next()
                        .ok_or("--seed needs a number")?
                        .parse::<u32>()
                        .map_err(|_| "--seed needs a u32 number")?,
                );
            }
            "--viewpoint" => {
                viewpoint = Some(PathBuf::from(
                    args.next().ok_or("--viewpoint needs a file")?,
                ))
            }
            _ => return Err(format!("Unknown option: {arg}")),
        }
    }
    let viewpoint_path = viewpoint
        .clone()
        .unwrap_or_else(|| PathBuf::from("viewpoint.bwv"));
    let mut game = seed.map_or_else(Game::new, Game::with_seed);
    if let Some(path) = &viewpoint {
        if path.exists() {
            Viewpoint::load(path)?.apply(&mut game, seed);
        } else {
            eprintln!("New viewpoint file: {} (press F5 to save)", path.display());
        }
    }
    let event_loop = EventLoop::new().map_err(|e| e.to_string())?;
    let mut attributes = Window::default_attributes()
        .with_title("BlockWorld")
        .with_inner_size(LogicalSize::new(1280, 800));
    if !windowed {
        attributes = attributes.with_fullscreen(Some(Fullscreen::Borderless(None)));
    }
    #[allow(deprecated)]
    let window = Arc::new(
        event_loop
            .create_window(attributes)
            .map_err(|e| e.to_string())?,
    );
    grab_cursor(&window);
    let mut renderer = pollster::block_on(Renderer::new(window.clone(), profile))?;
    let mut focused = true;
    let mut last_frame = Instant::now();
    let mut last_title = Instant::now();
    let mut frames = 0_u32;

    #[allow(deprecated)]
    event_loop
        .run(move |event, event_loop| {
            event_loop.set_control_flow(ControlFlow::Poll);
            match event {
                Event::DeviceEvent {
                    event: DeviceEvent::MouseMotion { delta },
                    ..
                } if focused => {
                    game.mouse_motion(delta.0, delta.1);
                }
                Event::WindowEvent { event, window_id } if window_id == window.id() => {
                    match event {
                        WindowEvent::CloseRequested => event_loop.exit(),
                        WindowEvent::Focused(is_focused) => {
                            focused = is_focused;
                            if focused {
                                grab_cursor(&window);
                            } else {
                                game.keys.clear();
                            }
                        }
                        WindowEvent::Resized(size) => renderer.resize(size.width, size.height),
                        WindowEvent::KeyboardInput { event, .. } => {
                            if let PhysicalKey::Code(key) = event.physical_key {
                                if event.state == ElementState::Pressed {
                                    if key == KeyCode::Escape {
                                        event_loop.exit();
                                    }
                                    if !event.repeat {
                                        match key {
                                            KeyCode::F5 => {
                                                if let Err(error) = Viewpoint::from_game(&game)
                                                    .save(&viewpoint_path)
                                                {
                                                    eprintln!("{error}");
                                                } else {
                                                    eprintln!(
                                                        "Saved viewpoint to {}",
                                                        viewpoint_path.display()
                                                    );
                                                }
                                            }
                                            KeyCode::F9 => {
                                                if let Err(error) = Viewpoint::load(&viewpoint_path)
                                                    .map(|viewpoint| {
                                                        viewpoint.apply(&mut game, seed)
                                                    })
                                                {
                                                    eprintln!("{error}");
                                                }
                                            }
                                            _ => game.key_pressed(key),
                                        }
                                    }
                                    game.keys.insert(key);
                                } else {
                                    game.keys.remove(&key);
                                }
                            }
                        }
                        WindowEvent::RedrawRequested => {
                            let now = Instant::now();
                            let dt = now.duration_since(last_frame).as_secs_f32();
                            last_frame = now;
                            game.update(dt);
                            renderer.update_chunks(&game.terrain, game.position.x, game.position.z);
                            match renderer.draw(&game) {
                                Ok(()) => {}
                                Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                                    let size = window.inner_size();
                                    renderer.resize(size.width, size.height);
                                }
                                Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                                Err(wgpu::SurfaceError::Timeout | wgpu::SurfaceError::Other) => {}
                            }
                            frames += 1;
                            if now.duration_since(last_title) >= Duration::from_secs(1) {
                                let elapsed = now.duration_since(last_title).as_secs_f32();
                                let mode = if game.walking {
                                    "Walk (G to fly)"
                                } else {
                                    "Flight (G to walk)"
                                };
                                let clock = if game.time_paused {
                                    " | Sun paused"
                                } else {
                                    ""
                                };
                                window.set_title(&format!(
                                    "BlockWorld | seed {} | {mode}{clock} | {:.0} FPS | {}, {}, {}",
                                    game.terrain.seed(),
                                    frames as f32 / elapsed,
                                    game.position.x.round() as i32,
                                    game.position.y.round() as i32,
                                    game.position.z.round() as i32,
                                ));
                                frames = 0;
                                last_title = now;
                            }
                        }
                        _ => {}
                    }
                }
                Event::AboutToWait => window.request_redraw(),
                _ => {}
            }
        })
        .map_err(|e| e.to_string())
}

fn grab_cursor(window: &Window) {
    if window.set_cursor_grab(CursorGrabMode::Locked).is_err() {
        let _ = window.set_cursor_grab(CursorGrabMode::Confined);
    }
    window.set_cursor_visible(false);
}
