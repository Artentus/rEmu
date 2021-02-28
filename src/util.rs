use crate::video::Color;
use packed_simd::f32x4;
use std::cmp::min;
use std::path::Path;

pub struct BinReader {
    data: Vec<u8>,
    pos: usize,
}
impl BinReader {
    const fn new(data: Vec<u8>) -> Self {
        Self { data, pos: 0 }
    }

    pub fn from_file<P: AsRef<Path>>(file: P) -> Result<Self, std::io::Error> {
        let data = std::fs::read(file)?;
        Ok(Self::new(data))
    }

    pub fn read_byte(&mut self) -> Option<u8> {
        if self.pos < self.data.len() {
            let byte = self.data[self.pos];
            self.pos += 1;
            Some(byte)
        } else {
            None
        }
    }

    pub fn read_into(&mut self, target: &mut [u8]) -> usize {
        let count = min(target.len(), self.data.len() - self.pos);
        if count > 0 {
            target.copy_from_slice(&self.data[self.pos..(self.pos + count)]);
            self.pos += count;
        }
        count
    }

    pub fn skip(&mut self, count: usize) {
        self.pos += count;
    }
}

pub fn pixels_to_data(pixels: &[Color]) -> &[u8] {
    const COLOR_SIZE: usize = std::mem::size_of::<Color>();

    unsafe {
        let pixel_ptr = pixels.as_ptr();
        let data_ptr = pixel_ptr as *const u8;
        let len = pixels.len() * COLOR_SIZE;
        std::slice::from_raw_parts(data_ptr, len)
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct ColorYuv(u8, u8, u8);
impl ColorYuv {
    #[inline]
    pub const fn y(&self) -> u8 {
        self.0
    }
    #[inline]
    pub const fn u(&self) -> u8 {
        self.0
    }
    #[inline]
    pub const fn v(&self) -> u8 {
        self.0
    }
}

pub fn rgb_to_yuv(r: u8, g: u8, b: u8) -> ColorYuv {
    const Y_FACTORS: f32x4 = f32x4::new(0.257, 0.504, 0.098, 16.0);
    const U_FACTORS: f32x4 = f32x4::new(-0.148, -0.291, 0.439, 128.0);
    const V_FACTORS: f32x4 = f32x4::new(0.439, -0.368, -0.071, 128.0);

    let rf = r as f32;
    let gf = g as f32;
    let bf = b as f32;
    let rgb: f32x4 = f32x4::new(rf, gf, bf, 1.0);

    let yf = (rgb * Y_FACTORS).sum();
    let uf = (rgb * U_FACTORS).sum();
    let vf = (rgb * V_FACTORS).sum();

    const MIN: f32 = u8::MIN as f32;
    const MAX: f32 = u8::MAX as f32;
    let y = yf.clamp(MIN, MAX) as u8;
    let u = uf.clamp(MIN, MAX) as u8;
    let v = vf.clamp(MIN, MAX) as u8;

    ColorYuv(y, u, v)
}

#[inline]
pub fn color_to_yuv(color: Color) -> ColorYuv {
    rgb_to_yuv(color.r(), color.g(), color.b())
}
