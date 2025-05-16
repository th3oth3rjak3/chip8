use chip8_core::*;
use sdl2::audio::{AudioCallback, AudioSpecDesired, AudioStatus};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;
use std::env;
use std::fs::File;
use std::io::Read;
use std::time::{Duration, Instant};

const SCALE: u32 = 15;
const WINDOW_WIDTH: u32 = (SCREEN_WIDTH as u32) * SCALE;
const WINDOW_HEIGHT: u32 = (SCREEN_HEIGHT as u32) * SCALE;
const TICKS_PER_FRAME: u32 = 1;

struct SquareWave {
    phase_inc: f32,
    phase: f32,
    volume: f32,
}

impl AudioCallback for SquareWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        // Generate a square wave
        for x in out.iter_mut() {
            *x = if self.phase <= 0.5 {
                self.volume
            } else {
                -self.volume
            };
            self.phase = (self.phase + self.phase_inc) % 1.0;
        }
    }
}

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: cargo run path/to/rom");
        return;
    }

    // Setup SDL
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let audio_subsystem = sdl_context.audio().unwrap();

    let desired_spec = AudioSpecDesired {
        freq: Some(44100),
        channels: Some(1), // mono
        samples: None,     // default sample size
    };

    // Set up the audio device
    let device = audio_subsystem
        .open_playback(None, &desired_spec, |spec| {
            let frequency = 440.0; // A4 note frequency in Hz
            let phase_inc = frequency / spec.freq as f32;

            SquareWave {
                phase_inc,
                phase: 0.0,
                volume: 0.25,
            }
        })
        .unwrap();

    let window = video_subsystem
        .window("Chip-8 Emulator", WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .opengl()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    canvas.clear();
    canvas.present();

    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut chip8 = Emulator::new();
    let mut rom = File::open(&args[1]).expect("Unable to open ROM");
    let mut buffer = Vec::new();
    rom.read_to_end(&mut buffer).expect("Unable to read ROM");
    chip8.load_rom(&buffer);

    let mut last_frame = Instant::now();

    'gameLoop: loop {
        for evt in event_pump.poll_iter() {
            match evt {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'gameLoop,
                Event::KeyDown {
                    keycode: Some(key), ..
                } => {
                    if let Some(k) = key2btn(key) {
                        chip8.keypress(k, true);
                    } else {
                        if key == Keycode::Space {
                            chip8.reset();
                            chip8.load_rom(&buffer);
                        }
                    }
                }
                Event::KeyUp {
                    keycode: Some(key), ..
                } => {
                    if let Some(k) = key2btn(key) {
                        chip8.keypress(k, false);
                    }
                }
                _ => (),
            }
        }

        for _ in 0..TICKS_PER_FRAME {
            if chip8.draw_completed {
                chip8.tick();
            }
        }

        match device.status() {
            AudioStatus::Playing => {},
            AudioStatus::Paused => {}
            AudioStatus::Stopped => {}
        }
        
        match device.status() {
            AudioStatus::Playing => {
                if chip8.st == 0 {
                    device.pause();
                }
            }
            AudioStatus::Paused | AudioStatus::Stopped => {
                if chip8.st > 0 {
                    device.resume();
                }
            }
        }
        
        if last_frame.elapsed() >= Duration::from_millis(16) {
            draw_screen(&chip8, &mut canvas);
            chip8.draw_completed = true;
            chip8.tick_timers();
            last_frame = Instant::now();
        }
    }
}

fn draw_screen(emulator: &Emulator, canvas: &mut Canvas<Window>) {
    // background black
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();

    let screen_buf = emulator.get_display();
    // set draw color to white to draw sprites
    canvas.set_draw_color(Color::RGB(255, 255, 255));

    for (i, pixel) in screen_buf.iter().enumerate() {
        if *pixel {
            // convert the 1d array into coordinates (x, y) position
            let x = (i % SCREEN_WIDTH) as u32;
            let y = (i / SCREEN_WIDTH) as u32;

            // Draw a rectangle at (x, y) scaled up by our scale value.
            let rect = Rect::new((x * SCALE) as i32, (y * SCALE) as i32, SCALE, SCALE);
            canvas.fill_rect(rect).unwrap();
        }
    }

    canvas.present();
}

fn key2btn(key: Keycode) -> Option<usize> {
    match key {
        Keycode::NUM_1 => Some(0x1),
        Keycode::NUM_2 => Some(0x2),
        Keycode::NUM_3 => Some(0x3),
        Keycode::NUM_4 => Some(0xC),
        Keycode::Q => Some(0x4),
        Keycode::W => Some(0x5),
        Keycode::E => Some(0x6),
        Keycode::R => Some(0xD),
        Keycode::A => Some(0x7),
        Keycode::S => Some(0x8),
        Keycode::D => Some(0x9),
        Keycode::F => Some(0xE),
        Keycode::Z => Some(0xA),
        Keycode::X => Some(0x0),
        Keycode::C => Some(0xB),
        Keycode::V => Some(0xF),
        _ => None,
    }
}
