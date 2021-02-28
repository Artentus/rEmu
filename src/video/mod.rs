#[allow(non_snake_case)]
pub mod ppu2C02;

use crate::*;
use bus::BusComponent;
use util::pixels_to_data;

#[repr(C)]
#[repr(align(4))]
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Color {
    pub channels: [u8; 4],
}
impl Color {
    pub const BLACK: Color = Color::from_rgb(u8::MIN, u8::MIN, u8::MIN);
    pub const WHITE: Color = Color::from_rgb(u8::MAX, u8::MAX, u8::MAX);
    pub const TRANSPARENT: Color = Color::from_rgba(u8::MIN, u8::MIN, u8::MIN, u8::MIN);

    #[inline]
    pub const fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            channels: [r, g, b, a],
        }
    }

    #[inline]
    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self {
            channels: [r, g, b, u8::MAX],
        }
    }

    #[inline]
    pub const fn r(&self) -> u8 {
        self.channels[0]
    }
    #[inline]
    pub const fn g(&self) -> u8 {
        self.channels[1]
    }
    #[inline]
    pub const fn b(&self) -> u8 {
        self.channels[2]
    }
    #[inline]
    pub const fn a(&self) -> u8 {
        self.channels[3]
    }

    #[inline]
    pub fn r_mut(&mut self) -> &mut u8 {
        &mut self.channels[0]
    }
    #[inline]
    pub fn g_mut(&mut self) -> &mut u8 {
        &mut self.channels[1]
    }
    #[inline]
    pub fn b_mut(&mut self) -> &mut u8 {
        &mut self.channels[2]
    }
    #[inline]
    pub fn a_mut(&mut self) -> &mut u8 {
        &mut self.channels[3]
    }
}

pub trait VideoBuffer {
    fn width(&self) -> usize;
    fn height(&self) -> usize;

    fn get_pixels(&self) -> &[Color];

    #[inline]
    fn get_pixel(&self, x: usize, y: usize) -> Color {
        let index = (y * self.width()) + x;
        self.get_pixels()[index]
    }

    fn get_pixel_data(&self) -> &[u8] {
        pixels_to_data(self.get_pixels())
    }
}

pub trait VideoChip<'a, TCpuAddress, TCpuWord, TAddress, TWord>:
    BusComponent<TCpuAddress, TCpuWord>
where
    TCpuAddress: HardwareInteger,
    TCpuWord: HardwareInteger,
    TAddress: HardwareInteger,
    TWord: HardwareInteger,
{
    fn get_buffer(&self) -> &(dyn VideoBuffer + 'a);

    fn reset(&mut self);
    fn clock(&mut self, cycles: u32);
}
