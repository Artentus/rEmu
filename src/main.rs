#![feature(const_fn)]
#![feature(const_panic)]

#[macro_use]
extern crate bitflags;

use audio::SampleBuffer;
use ggez::conf::{NumSamples, WindowMode, WindowSetup};
use ggez::event::{EventHandler, KeyCode};
use ggez::graphics::{DrawParam, FilterMode, Font, Image, WrapMode, PxScale};
#[allow(unused_imports)]
use ggez::graphics::{Text, TextFragment};
use ggez::{event, graphics, timer, Context, ContextBuilder, GameResult};
use scaler::Scaler;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::error::Error;
use std::fmt::Display;
use std::num::Wrapping;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use system::nes::*;
use util::pixels_to_data;
use video::Color;

pub mod audio;
pub mod bus;
pub mod cpu;
pub mod memory;
pub mod scaler;
pub mod system;
pub mod types;
pub mod util;
pub mod video;

const TITLE: &str = "rEmu";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");

// These should be adjustable but consts are fine for now
const SCREEN_SCALE: f32 = 4.0;
const ASPECT_RATIO: AspectRatio = AspectRatio::FourByThree;
const SCALER: Scaler = scaler::NONE;
const FILTER: FilterMode = FilterMode::Nearest;
const SHOW_DEBUG_INFO: bool = true;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
#[allow(dead_code)]
pub enum AspectRatio {
    SquarePixels,
    FourByThree,
}
impl AspectRatio {
    fn width_factor(self) -> f32 {
        match self {
            AspectRatio::SquarePixels => 1.0,
            AspectRatio::FourByThree => 1.25,
        }
    }
}

pub type EmuRef<T> = Rc<RefCell<T>>;

#[inline]
pub fn make_ref<T>(value: T) -> EmuRef<T> {
    Rc::new(RefCell::new(value))
}

#[inline]
pub fn clone_ref<T: ?Sized>(r: &EmuRef<T>) -> EmuRef<T> {
    Rc::clone(r)
}

#[derive(Debug)]
pub struct ArgError;
impl Display for ArgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Invalid arguments"))
    }
}
impl Error for ArgError {}

pub const FRAME_RATE: u32 = 60;
pub const SAMPLE_RATE: u32 = 44100;
pub const SECONDS_PER_SAMPLE: f32 = 1.0 / (SAMPLE_RATE as f32);

pub struct SampleBufferSource {
    buffer: Arc<Mutex<SampleBuffer>>,
    sample_queue: VecDeque<f32>,
}
impl SampleBufferSource {
    #[inline]
    pub fn new(buffer: Arc<Mutex<SampleBuffer>>) -> Self {
        Self {
            buffer,
            sample_queue: VecDeque::new(),
        }
    }
}
impl Iterator for SampleBufferSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.sample_queue.len() == 0 {
            let mut buffer_lock = self.buffer.lock().unwrap();
            loop {
                let sample_opt = buffer_lock.read();
                if let Some(sample) = sample_opt {
                    self.sample_queue.push_back(sample)
                } else {
                    break;
                }
            }
        }

        if let Some(sample) = self.sample_queue.pop_front() {
            Some(sample)
        } else {
            Some(0.0)
        }
    }
}
impl rodio::Source for SampleBufferSource {
    #[inline]
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    #[inline]
    fn channels(&self) -> u16 {
        1
    }
    #[inline]
    fn sample_rate(&self) -> u32 {
        SAMPLE_RATE
    }
    #[inline]
    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Box<[String]> = std::env::args().collect();
    if args.len() < 2 {
        Err(Box::new(ArgError))
    } else {
        let path = PathBuf::from(&args[1]);
        run_emu(path, SCREEN_SCALE, ASPECT_RATIO, SCALER, FILTER)?;

        Ok(())
    }
}

fn run_emu<P: AsRef<Path>>(
    cartridge_file: P,
    scale: f32,
    aspect_ratio: AspectRatio,
    scaler: Scaler,
    filter: FilterMode,
) -> Result<(), Box<dyn Error>> {
    let emu = Nes::new();

    let window_setup = WindowSetup::default()
        .title(&format!("{} v{}", TITLE, VERSION))
        .vsync(true)
        .srgb(true)
        .samples(NumSamples::One); // We draw 2D sprites only

    let (width, height) = {
        let screen_buffer = emu.screen();
        let w = (screen_buffer.width() * scaler.scale_factor()) as f32
            * scale
            * aspect_ratio.width_factor();
        let h = (screen_buffer.height() * scaler.scale_factor()) as f32 * scale;
        (w, h)
    };
    let window_mode = WindowMode::default().dimensions(width, height);

    let builder = ContextBuilder::new(TITLE, AUTHOR)
        .window_setup(window_setup)
        .window_mode(window_mode);
    let (mut ctx, event_loop) = builder.build()?;

    const FONT_BYTES: &[u8] = include_bytes!("../res/SourceCodePro-Bold.ttf");
    let font = Font::new_glyph_font_bytes(&mut ctx, FONT_BYTES)?;

    let (_stream, stream_handle) = rodio::OutputStream::try_default()?;
    let audio_buffer = Arc::new(Mutex::new(SampleBuffer::new(1024 * 1024)));
    let audio_source = SampleBufferSource::new(Arc::clone(&audio_buffer));
    stream_handle.play_raw(audio_source)?;

    let state = EmuState::new(
        emu,
        scale,
        aspect_ratio,
        scaler,
        filter,
        font,
        audio_buffer,
        cartridge_file,
    );

    event::run(ctx, event_loop, state)
}

struct EmuState<'a> {
    emu: Nes<'a>,
    scale: [f32; 2],
    scaler: Scaler,
    filter: FilterMode,
    #[allow(dead_code)]
    cartridge: Rc<RefCell<Cartridge>>,
    controller_0: Buttons,
    controller_1: Buttons,
    scaler_output_buffer: Option<Box<[Color]>>,
    font: Font,
    audio_buffer: Arc<Mutex<SampleBuffer>>,
    run: bool,
}
impl<'a> EmuState<'a> {
    pub fn new<P: AsRef<Path>>(
        mut emu: Nes<'a>,
        scale: f32,
        aspect_ratio: AspectRatio,
        scaler: Scaler,
        filter: FilterMode,
        font: Font,
        audio_buffer: Arc<Mutex<SampleBuffer>>,
        cartridge_file: P,
    ) -> Self {
        let cartridge = load_cartridge(cartridge_file).expect("Invalid cartridge file");

        emu.set_cartridge(clone_ref(&cartridge));
        emu.reset();

        Self {
            emu,
            scale: [scale as f32 * aspect_ratio.width_factor(), scale as f32],
            scaler,
            filter,
            cartridge,
            controller_0: Buttons::empty(),
            controller_1: Buttons::empty(),
            scaler_output_buffer: None,
            font,
            audio_buffer,
            run: true,
        }
    }
}
impl<'a> EventHandler for EmuState<'a> {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        self.emu
            .update_input_state(self.controller_0, self.controller_1);

        while timer::check_update_time(ctx, FRAME_RATE) {
            if self.run {
                let mut locked_buffer = self.audio_buffer.lock().unwrap();
                self.emu.next_frame(&mut locked_buffer);
            }
        }

        graphics::set_window_title(
            ctx,
            &format!("{} v{} - {:.1} fps", TITLE, VERSION, timer::fps(ctx)),
        );

        timer::yield_now();
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        graphics::clear(ctx, ggez::graphics::Color::BLACK);

        let screen_buffer = self.emu.screen();
        let screen_width = screen_buffer.width();
        let screen_height = screen_buffer.height();
        let pixel_buffer = screen_buffer.get_pixels();

        let output_buffer_ref = &mut self.scaler_output_buffer;

        let scaled_buffer_size =
            pixel_buffer.len() * self.scaler.scale_factor() * self.scaler.scale_factor();
        if let Some(scaled_pixel_buffer) = output_buffer_ref {
            if scaled_pixel_buffer.len() != scaled_buffer_size {
                std::mem::drop(output_buffer_ref);
                self.scaler_output_buffer =
                    Some(vec![Color::BLACK; scaled_buffer_size].into_boxed_slice());
            }
        } else {
            std::mem::drop(output_buffer_ref);
            self.scaler_output_buffer =
                Some(vec![Color::BLACK; scaled_buffer_size].into_boxed_slice());
        }

        let scaled_screen_width = screen_width * self.scaler.scale_factor();
        let scaled_screen_height = screen_height * self.scaler.scale_factor();

        let output_buffer_ref = &mut self.scaler_output_buffer;
        if let Some(scaled_pixel_buffer) = output_buffer_ref {
            self.scaler.scale(
                pixel_buffer,
                scaled_pixel_buffer,
                screen_width,
                screen_height,
            );

            let mut screen = Image::from_rgba8(
                ctx,
                scaled_screen_width as u16,
                scaled_screen_height as u16,
                pixels_to_data(&scaled_pixel_buffer),
            )?;
            screen.set_filter(self.filter);
            screen.set_wrap(WrapMode::Clamp, WrapMode::Clamp);

            let params = DrawParam::default().dest([0.0, 0.0]).scale(self.scale);
            graphics::draw(ctx, &screen, params)?;
        }

        if SHOW_DEBUG_INFO {
            const TEXT_SCALE: PxScale = PxScale { x: 20.0, y: 20.0 };
            const TEXT_BACK_COLOR: graphics::Color = graphics::Color::new(0.0, 0.0, 0.0, 1.0);
            const TEXT_FRONT_COLOR: graphics::Color = graphics::Color::new(0.5, 1.0, 0.0, 1.0);

            let emu_info = format!("{}", self.emu);
            let emu_info_frag = TextFragment::new(emu_info)
                .font(self.font)
                .scale(TEXT_SCALE);
            let emu_info_text = Text::new(emu_info_frag);
            graphics::draw(
                ctx,
                &emu_info_text,
                DrawParam::default()
                    .dest([11.0, 11.0])
                    .color(TEXT_BACK_COLOR),
            )?;
            graphics::draw(
                ctx,
                &emu_info_text,
                DrawParam::default()
                    .dest([10.0, 10.0])
                    .color(TEXT_FRONT_COLOR),
            )?;
        }

        graphics::present(ctx)?;
        timer::yield_now();
        Ok(())
    }

    // Input handling currently only supports one virtual controller

    fn key_down_event(
        &mut self,
        ctx: &mut Context,
        keycode: event::KeyCode,
        _keymods: event::KeyMods,
        _repeat: bool,
    ) {
        match keycode {
            KeyCode::Escape => event::quit(ctx),
            KeyCode::Up => self.controller_0.insert(Buttons::UP),
            KeyCode::Left => self.controller_0.insert(Buttons::LEFT),
            KeyCode::Down => self.controller_0.insert(Buttons::DOWN),
            KeyCode::Right => self.controller_0.insert(Buttons::RIGHT),
            KeyCode::Q => self.controller_0.insert(Buttons::SELECT),
            KeyCode::W => self.controller_0.insert(Buttons::START),
            KeyCode::E => self.controller_0.insert(Buttons::B),
            KeyCode::R => self.controller_0.insert(Buttons::A),
            KeyCode::Space => self.run = !self.run,
            KeyCode::S => {
                if !self.run {
                    let mut locked_buffer = self.audio_buffer.lock().unwrap();
                    self.emu.next_instruction(&mut locked_buffer);
                }
            }
            _ => {}
        }
    }

    fn key_up_event(&mut self, _ctx: &mut Context, keycode: KeyCode, _keymods: event::KeyMods) {
        match keycode {
            KeyCode::Up => self.controller_0.remove(Buttons::UP),
            KeyCode::Left => self.controller_0.remove(Buttons::LEFT),
            KeyCode::Down => self.controller_0.remove(Buttons::DOWN),
            KeyCode::Right => self.controller_0.remove(Buttons::RIGHT),
            KeyCode::Q => self.controller_0.remove(Buttons::SELECT),
            KeyCode::W => self.controller_0.remove(Buttons::START),
            KeyCode::E => self.controller_0.remove(Buttons::B),
            KeyCode::R => self.controller_0.remove(Buttons::A),
            _ => {}
        }
    }

    fn gamepad_button_down_event(
        &mut self,
        _ctx: &mut Context,
        btn: event::Button,
        _id: event::GamepadId,
    ) {
        match btn {
            event::Button::DPadUp => self.controller_0.insert(Buttons::UP),
            event::Button::DPadLeft => self.controller_0.insert(Buttons::LEFT),
            event::Button::DPadDown => self.controller_0.insert(Buttons::DOWN),
            event::Button::DPadRight => self.controller_0.insert(Buttons::RIGHT),
            event::Button::Select => self.controller_0.insert(Buttons::SELECT),
            event::Button::Start => self.controller_0.insert(Buttons::START),
            // These assignments create a layout identical to most games on new Nintendo consoles
            event::Button::North => self.controller_0.insert(Buttons::B), // Y on XBox gamepads
            event::Button::East => self.controller_0.insert(Buttons::A),  // B on XBox gamepads
            event::Button::South => self.controller_0.insert(Buttons::A), // A on XBox gamepads
            event::Button::West => self.controller_0.insert(Buttons::B),  // X on XBox gamepads
            _ => {}
        }
    }

    fn gamepad_button_up_event(
        &mut self,
        _ctx: &mut Context,
        btn: event::Button,
        _id: event::GamepadId,
    ) {
        match btn {
            event::Button::DPadUp => self.controller_0.remove(Buttons::UP),
            event::Button::DPadLeft => self.controller_0.remove(Buttons::LEFT),
            event::Button::DPadDown => self.controller_0.remove(Buttons::DOWN),
            event::Button::DPadRight => self.controller_0.remove(Buttons::RIGHT),
            event::Button::Select => self.controller_0.remove(Buttons::SELECT),
            event::Button::Start => self.controller_0.remove(Buttons::START),
            // These assignments create a layout identical to most games on new Nintendo consoles
            event::Button::North => self.controller_0.remove(Buttons::B), // Y on XBox gamepads
            event::Button::East => self.controller_0.remove(Buttons::A),  // B on XBox gamepads
            event::Button::South => self.controller_0.remove(Buttons::A), // A on XBox gamepads
            event::Button::West => self.controller_0.remove(Buttons::B),  // X on XBox gamepads
            _ => {}
        }
    }
}
