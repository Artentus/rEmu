#[allow(non_snake_case)]
pub mod apu2A03;

use crate::bus::BusComponent;
use crate::types::HardwareInteger;

pub type Sample = f32;

pub struct SampleBuffer {
    size: usize,
    start: usize,
    end: usize,
    len: usize,
    samples: Vec<Sample>,
}
unsafe impl Send for SampleBuffer {}
unsafe impl Sync for SampleBuffer {}
impl SampleBuffer {
    #[inline]
    pub fn new(size: usize) -> Self {
        Self {
            samples: vec![0.0; size],
            size,
            start: 0,
            end: 0,
            len: 0,
        }
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.len
    }

    pub fn read(&mut self) -> Option<Sample> {
        if self.len > 0 {
            let sample = self.samples[self.start];
            self.start = (self.start + 1) % self.size;
            self.len -= 1;
            Some(sample)
        } else {
            None
        }
    }

    pub fn write(&mut self, sample: Sample) {
        self.samples[self.end] = sample;
        self.end = (self.end + 1) % self.size;
        self.len += 1;

        if self.len > self.size {
            panic!("Buffer overflow")
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.start = 0;
        self.end = 0;
        self.len = 0;
    }

    pub fn copy_to(&mut self, buffer: &mut [f32]) {
        for i in 0..self.len {
            buffer[i] = self.samples[(self.start + i) % self.len];
        }
        self.clear();
    }
}

pub trait AudioChip<'a, TAddress, TWord>: BusComponent<TAddress, TWord>
where
    TAddress: HardwareInteger,
    TWord: HardwareInteger,
{
    fn reset(&mut self);
    fn clock(&mut self, cycles: u32, buffer: &mut SampleBuffer);
}
