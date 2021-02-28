use crate::video::Color;

pub mod hqx;

pub type ScalerFn = fn(
    source_buffer: &[Color],
    target_buffer: &mut [Color],
    source_width: usize,
    source_height: usize,
);

#[derive(Clone, Copy)]
pub struct Scaler {
    function: ScalerFn,
    scale_factor: usize,
}
impl Scaler {
    #[inline]
    pub const fn scale_factor(&self) -> usize {
        self.scale_factor
    }

    #[inline]
    pub fn scale(
        &self,
        source_buffer: &[Color],
        target_buffer: &mut [Color],
        source_width: usize,
        source_height: usize,
    ) {
        (self.function)(source_buffer, target_buffer, source_width, source_height);
    }
}

pub const NONE: Scaler = Scaler {
    function: no_scaler,
    scale_factor: 1,
};

fn no_scaler(
    source_buffer: &[Color],
    target_buffer: &mut [Color],
    _source_width: usize,
    _source_height: usize,
) {
    target_buffer.copy_from_slice(source_buffer);
}
