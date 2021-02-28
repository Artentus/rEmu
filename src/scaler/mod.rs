use crate::video::Color;

pub mod hqx;

pub type Scaler = fn(
    source_buffer: &[Color],
    target_buffer: &mut [Color],
    source_width: usize,
    source_height: usize,
);

pub const NONE: Scaler = no_scaler;

fn no_scaler(
    source_buffer: &[Color],
    target_buffer: &mut [Color],
    _source_width: usize,
    _source_height: usize,
) {
    target_buffer.copy_from_slice(source_buffer);
}
