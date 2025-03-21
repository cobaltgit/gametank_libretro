#![allow(unused)]

use std::collections::HashMap;

#[macro_use]
use libretro_rs::prelude::*;

use std::ffi::c_uint;
use std::time::Instant;
use gametank_emulator_core::color_map::COLOR_MAP;
use gametank_emulator_core::emulator::{Emulator, PlayState, TimeDaemon};
use gametank_emulator_core::inputs::{ControllerButton, InputCommand, KeyState};
use gametank_emulator_core::inputs::InputCommand::{Controller1, Controller2};
use libretro_rs::prelude::env::{GetAvInfo, Init, Reset, Run, UnloadGame};

struct CoreEmulator {
    emu: Emulator<InstantClock>,
    audio_consumer: rtrb::Consumer<u8>,
    rendering_mode: Option<SoftwareRenderEnabled>,
    // game_data: Option<GameData>,
    input_bindings: HashMap<(c_uint, JoypadButton), InputCommand>,
    pixel_format: Option<ActiveFormat<XRGB8888>>,
    framebuffer: FrameBufferThing,
}

struct FrameBufferThing {
    video_frame: Vec<u8>
}

struct InstantClock {
    instant: Instant,
}

impl TimeDaemon for InstantClock {
    fn get_now_ms(&self) -> f64 {
        self.instant.elapsed().as_millis() as f64
    }
}

impl Default for CoreEmulator {
    fn default() -> Self {
        let clock = InstantClock { instant: Instant::now() };
        // if you go past 4096, you're boned anyway
        let (producer, consumer) = rtrb::RingBuffer::new(4096);

        let mut input_bindings = HashMap::new();

        input_bindings.insert((0, JoypadButton::Start), Controller1(ControllerButton::Start));
        input_bindings.insert((0, JoypadButton::Up), Controller1(ControllerButton::Up));
        input_bindings.insert((0, JoypadButton::Down), Controller1(ControllerButton::Down));
        input_bindings.insert((0, JoypadButton::Left), Controller1(ControllerButton::Left));
        input_bindings.insert((0, JoypadButton::Right), Controller1(ControllerButton::Right));
        input_bindings.insert((0, JoypadButton::A), Controller1(ControllerButton::A));
        input_bindings.insert((0, JoypadButton::B), Controller1(ControllerButton::B));
        input_bindings.insert((0, JoypadButton::X), Controller1(ControllerButton::C));

        input_bindings.insert((1, JoypadButton::Start), Controller2(ControllerButton::Start));
        input_bindings.insert((1, JoypadButton::Up), Controller2(ControllerButton::Up));
        input_bindings.insert((1, JoypadButton::Down), Controller2(ControllerButton::Down));
        input_bindings.insert((1, JoypadButton::Left), Controller2(ControllerButton::Left));
        input_bindings.insert((1, JoypadButton::Right), Controller2(ControllerButton::Right));
        input_bindings.insert((1, JoypadButton::A), Controller2(ControllerButton::A));
        input_bindings.insert((1, JoypadButton::B), Controller2(ControllerButton::B));
        input_bindings.insert((1, JoypadButton::X), Controller2(ControllerButton::C));

        Self {
            emu: Emulator::init(clock, producer),
            audio_consumer: consumer,
            // game_data: None,
            input_bindings,
            rendering_mode: None,
            pixel_format: None,
            framebuffer: FrameBufferThing { video_frame: vec![] },
        }
    }
}

pub fn buffer_to_color_image(framebuffer: &[u8; 128*128]) -> Vec<u8> {
    let mut pixels: Vec<u8> = Vec::with_capacity(128 * 128 * 4); // 4 channels per pixel (RGBA)

    for &index in framebuffer.iter() {
        let (r, g, b, a) = COLOR_MAP[index as usize];
        pixels.push(b);
        pixels.push(g);
        pixels.push(r);
        pixels.push(a);
    }

    pixels
}

impl<'a> Core<'a> for CoreEmulator {
    type Init = Self;

    fn get_system_info() -> SystemInfo {
        SystemInfo::new(
            c_utf8!("GameTank Rust!"),
            c_utf8!("1.69.422"),
            Extensions::new(c_utf8!("gtr")),
        )
    }

    fn init(env: &mut impl Init) -> Self::Init {
        Self::default()
    }

    fn load_game<E: env::LoadGame>(
        game: &GameInfo,
        args: LoadGameExtraArgs<'a, '_, E, Self::Init>,
    ) -> Result<Self, CoreError> {
        let LoadGameExtraArgs { env, pixel_format, rendering_mode, .. } = args;
        let pixel_format = env.set_pixel_format_xrgb8888(pixel_format)?;
        let game_data = unsafe { game.as_data_unchecked() };

        let game_slice = game_data.data();

        let mut core = Self::default();
        core.emu.load_rom(game_slice);
        // core.game_data = Some(game_data);
        core.emu.play_state = PlayState::Playing;
        core.rendering_mode = Some(rendering_mode);
        core.pixel_format = Some(pixel_format);

        Ok(core)
    }

    fn get_system_av_info(&self, env: &mut impl GetAvInfo) -> SystemAVInfo {
        // default timing is 60FPS, 44.1KHz
        SystemAVInfo::default_timings(GameGeometry::fixed(128, 128))
    }

    fn run(&mut self, env: &mut impl Run, callbacks: &mut impl Callbacks) -> InputsPolled {
        let inputs_polled = callbacks.poll_inputs();
        // update emulator inputs
        for ((port, button), command) in &self.input_bindings {
            if let Some(ks) = self.emu.input_state.get(&command) {
                self.emu.set_input_state(*command, ks.update_state(callbacks.is_joypad_button_pressed(DevicePort::new(*port), *button)))
            } else {
                self.emu.set_input_state(*command, KeyState::new(callbacks.is_joypad_button_pressed(DevicePort::new(*port), *button)))
            }
        }

        while !self.audio_consumer.is_empty() {
            let _ignored = self.audio_consumer.pop();
        }
        
        self.emu.process_cycles(false);


        let framebuffer = self.emu.cpu_bus.read_full_framebuffer();
        self.framebuffer.video_frame = buffer_to_color_image(&framebuffer);

        let rendering_mode = self.rendering_mode.take().unwrap();
        let pixel_format = self.pixel_format.take().unwrap();
        
        callbacks.upload_video_frame(&rendering_mode, &pixel_format, &self.framebuffer);
        self.rendering_mode = Some(rendering_mode);
        self.pixel_format = Some(pixel_format);

        inputs_polled
    }

    fn reset(&mut self, env: &mut impl Reset) {
        todo!()
    }

    fn unload_game(self, env: &mut impl UnloadGame) -> Self::Init {
        todo!()
    }
}

unsafe impl FrameBuffer for FrameBufferThing {
    type Pixel = XRGB8888;

    fn data(&self) -> &[u8] {
        &self.video_frame
    }

    fn width(&self) -> u16 {
        128
    }

    fn height(&self) -> u16 {
        128
    }
}

libretro_core!( crate::CoreEmulator );
