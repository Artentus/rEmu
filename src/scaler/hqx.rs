use crate::scaler::Scaler;
use crate::util::{color_to_yuv, ColorYuv};
use crate::video::Color;
use packed_simd::{i32x4, m32x4, u32x4, u8x4};
use rayon::prelude::*;

pub const HQ2X: Scaler = hq2x;
pub const HQ2X_SCALING_FACTOR: usize = 2;

pub const HQ3X: Scaler = hq3x;
pub const HQ3X_SCALING_FACTOR: usize = 3;

pub const HQ4X: Scaler = hq4x;
pub const HQ4X_SCALING_FACTOR: usize = 4;

fn yuv_diff(yuv1: ColorYuv, yuv2: ColorYuv) -> bool {
    const THRESHOLD: i32x4 = i32x4::new(0x00000030, 0x00000007, 0x00000006, i32::MAX);
    const ZERO: i32x4 = i32x4::new(0, 0, 0, 0);
    const MINUS_ONE: i32x4 = i32x4::new(-1, -1, -1, -1);

    let a: i32x4 = u8x4::new(yuv1.y(), yuv1.u(), yuv1.v(), 0).into();
    let b: i32x4 = u8x4::new(yuv2.y(), yuv2.u(), yuv2.v(), 0).into();

    let a_minus_b: i32x4 = a - b;
    let abs_m: m32x4 = a_minus_b.lt(ZERO);
    let abs: i32x4 = abs_m.select(a_minus_b * MINUS_ONE, a_minus_b);

    abs.gt(THRESHOLD).any()
}

#[inline]
fn color_diff(color1: Color, color2: Color) -> bool {
    yuv_diff(color_to_yuv(color1), color_to_yuv(color2))
}

fn interpolate_2(color1: Color, weight1: u32, color2: Color, weight2: u32, shift: u32) -> Color {
    if color1 == color2 {
        color1
    } else {
        const MASK: u32x4 = u32x4::new(0x000000FF, 0x000000FF, 0x000000FF, 0x000000FF);

        let c1: u32x4 = u8x4::from_slice_aligned(&color1.channels).into();
        let c2: u32x4 = u8x4::from_slice_aligned(&color2.channels).into();
        let w1: u32x4 = u32x4::splat(weight1);
        let w2: u32x4 = u32x4::splat(weight2);
        let s: u32x4 = u32x4::splat(shift);

        let r: u32x4 = (((c1 * w1) + (c2 * w2)) >> s) & MASK;
        Color::from_rgba(
            r.extract(0) as u8,
            r.extract(1) as u8,
            r.extract(2) as u8,
            r.extract(3) as u8,
        )
    }
}

fn interpolate_3(
    color1: Color,
    weight1: u32,
    color2: Color,
    weight2: u32,
    color3: Color,
    weight3: u32,
    shift: u32,
) -> Color {
    const MASK: u32x4 = u32x4::new(0x000000FF, 0x000000FF, 0x000000FF, 0x000000FF);

    let c1: u32x4 = u8x4::from_slice_aligned(&color1.channels).into();
    let c2: u32x4 = u8x4::from_slice_aligned(&color2.channels).into();
    let c3: u32x4 = u8x4::from_slice_aligned(&color3.channels).into();
    let w1: u32x4 = u32x4::splat(weight1);
    let w2: u32x4 = u32x4::splat(weight2);
    let w3: u32x4 = u32x4::splat(weight3);
    let s: u32x4 = u32x4::splat(shift);

    let r: u32x4 = (((c1 * w1) + (c2 * w2) + (c3 * w3)) >> s) & MASK;
    Color::from_rgba(
        r.extract(0) as u8,
        r.extract(1) as u8,
        r.extract(2) as u8,
        r.extract(3) as u8,
    )
}

#[inline]
fn interp1(color1: Color, color2: Color) -> Color {
    // (c1*3+c2)/4;
    interpolate_2(color1, 3, color2, 1, 2)
}

#[inline]
fn interp2(color1: Color, color2: Color, color3: Color) -> Color {
    // (c1*2+c2+c3)/4;
    interpolate_3(color1, 2, color2, 1, color3, 1, 2)
}

#[inline]
fn interp3(color1: Color, color2: Color) -> Color {
    // (c1*7+c2)/8;
    interpolate_2(color1, 7, color2, 1, 3)
}

#[inline]
fn interp4(color1: Color, color2: Color, color3: Color) -> Color {
    // (c1*2+(c2+c3)*7)/16;
    interpolate_3(color1, 2, color2, 7, color3, 7, 4)
}

#[inline]
fn interp5(color1: Color, color2: Color) -> Color {
    // (c1+c2)/2;
    interpolate_2(color1, 1, color2, 1, 1)
}

#[inline]
fn interp6(color1: Color, color2: Color, color3: Color) -> Color {
    // (c1*5+c2*2+c3)/8;
    interpolate_3(color1, 5, color2, 2, color3, 1, 3)
}

#[inline]
fn interp7(color1: Color, color2: Color, color3: Color) -> Color {
    // (c1*6+c2+c3)/8;
    interpolate_3(color1, 6, color2, 1, color3, 1, 3)
}

#[inline]
fn interp8(color1: Color, color2: Color) -> Color {
    // (c1*5+c2*3)/8;
    interpolate_2(color1, 5, color2, 3, 3)
}

#[inline]
fn interp9(color1: Color, color2: Color, color3: Color) -> Color {
    // (c1*2+(c2+c3)*3)/8;
    interpolate_3(color1, 2, color2, 3, color3, 3, 3)
}

#[inline]
fn interp10(color1: Color, color2: Color, color3: Color) -> Color {
    // (c1*14+c2+c3)/16;
    interpolate_3(color1, 14, color2, 1, color3, 1, 4)
}

type HqxFn = fn(
    w: &[Color; 10],
    target_buffer: &mut [Color],
    offset: usize,
    pattern: i32,
    dest_x: usize,
    dest_y: usize,
    source_width: usize,
);

fn hqx(
    factor: usize,
    source_buffer: &[Color],
    target_buffer: &mut [Color],
    source_width: usize,
    source_height: usize,
    inner_function: HqxFn,
) {
    let get_source_pixel = |x: usize, y: usize| {
        let xc = x.clamp(0, source_width - 1);
        let yc = y.clamp(0, source_height - 1);
        let index = (yc * source_width) + xc;
        source_buffer[index]
    };

    let target_chunks = target_buffer.par_chunks_exact_mut(source_width * factor * factor);
    target_chunks.enumerate().for_each(|(y, target)| {
        for x in 0..source_width {
            //   +----+----+----+
            //   |    |    |    |
            //   | w1 | w2 | w3 |
            //   +----+----+----+
            //   |    |    |    |
            //   | w4 | w5 | w6 |
            //   +----+----+----+
            //   |    |    |    |
            //   | w7 | w8 | w9 |
            //   +----+----+----+

            let mut w: [Color; 10] = [Color::BLACK; 10];
            w[1] = get_source_pixel(x - 1, y - 1);
            w[2] = get_source_pixel(x, y - 1);
            w[3] = get_source_pixel(x + 1, y - 1);
            w[4] = get_source_pixel(x - 1, y);
            w[5] = get_source_pixel(x, y);
            w[6] = get_source_pixel(x + 1, y);
            w[7] = get_source_pixel(x - 1, y + 1);
            w[8] = get_source_pixel(x, y + 1);
            w[9] = get_source_pixel(x + 1, y + 1);

            let mut pattern = 0x00;
            let mut flag = 0x01;

            let yuv1 = color_to_yuv(w[5]);

            for i in 1..10 {
                if i == 5 {
                    continue;
                }

                if w[i] != w[5] {
                    let yuv2 = color_to_yuv(w[i]);
                    if yuv_diff(yuv1, yuv2) {
                        pattern |= flag;
                    }
                }
                flag <<= 1;
            }

            let dest_x = x * factor;
            let dest_y = y * factor;
            let offset = y * source_width * factor * factor;
            inner_function(&w, target, offset, pattern, dest_x, dest_y, source_width);
        }
    });
}

#[inline]
fn hq2x(
    source_buffer: &[Color],
    target_buffer: &mut [Color],
    source_width: usize,
    source_height: usize,
) {
    hqx(
        HQ2X_SCALING_FACTOR,
        source_buffer,
        target_buffer,
        source_width,
        source_height,
        hq2x_inner,
    );
}

#[inline]
fn hq3x(
    source_buffer: &[Color],
    target_buffer: &mut [Color],
    source_width: usize,
    source_height: usize,
) {
    hqx(
        HQ3X_SCALING_FACTOR,
        source_buffer,
        target_buffer,
        source_width,
        source_height,
        hq3x_inner,
    );
}

#[inline]
fn hq4x(
    source_buffer: &[Color],
    target_buffer: &mut [Color],
    source_width: usize,
    source_height: usize,
) {
    hqx(
        HQ4X_SCALING_FACTOR,
        source_buffer,
        target_buffer,
        source_width,
        source_height,
        hq4x_inner,
    );
}

/*
    Scary code below, read at your own risk
*/

fn hq2x_inner(
    w: &[Color; 10],
    target_buffer: &mut [Color],
    offset: usize,
    pattern: i32,
    dest_x: usize,
    dest_y: usize,
    source_width: usize,
) {
    let mut set_target_pixel = |x: usize, y: usize, c: Color| {
        let index = (y * source_width * 2) + x;
        target_buffer[index - offset] = c;
    };

    macro_rules! pixel00_0 {
        () => {
            set_target_pixel(dest_x, dest_y, w[5]);
        };
    }
    macro_rules! pixel00_10 {
        () => {
            set_target_pixel(dest_x, dest_y, interp1(w[5], w[1]));
        };
    }
    macro_rules! pixel00_11 {
        () => {
            set_target_pixel(dest_x, dest_y, interp1(w[5], w[4]));
        };
    }
    macro_rules! pixel00_12 {
        () => {
            set_target_pixel(dest_x, dest_y, interp1(w[5], w[2]));
        };
    }
    macro_rules! pixel00_20 {
        () => {
            set_target_pixel(dest_x, dest_y, interp2(w[5], w[4], w[2]));
        };
    }
    macro_rules! pixel00_21 {
        () => {
            set_target_pixel(dest_x, dest_y, interp2(w[5], w[1], w[2]));
        };
    }
    macro_rules! pixel00_22 {
        () => {
            set_target_pixel(dest_x, dest_y, interp2(w[5], w[1], w[4]));
        };
    }
    macro_rules! pixel00_60 {
        () => {
            set_target_pixel(dest_x, dest_y, interp6(w[5], w[2], w[4]));
        };
    }
    macro_rules! pixel00_61 {
        () => {
            set_target_pixel(dest_x, dest_y, interp6(w[5], w[4], w[2]));
        };
    }
    macro_rules! pixel00_70 {
        () => {
            set_target_pixel(dest_x, dest_y, interp7(w[5], w[4], w[2]));
        };
    }
    macro_rules! pixel00_90 {
        () => {
            set_target_pixel(dest_x, dest_y, interp9(w[5], w[4], w[2]));
        };
    }
    macro_rules! pixel00_100 {
        () => {
            set_target_pixel(dest_x, dest_y, interp10(w[5], w[4], w[2]));
        };
    }

    macro_rules! pixel01_0 {
        () => {
            set_target_pixel(dest_x + 1, dest_y, w[5]);
        };
    }
    macro_rules! pixel01_10 {
        () => {
            set_target_pixel(dest_x + 1, dest_y, interp1(w[5], w[3]));
        };
    }
    macro_rules! pixel01_11 {
        () => {
            set_target_pixel(dest_x + 1, dest_y, interp1(w[5], w[2]));
        };
    }
    macro_rules! pixel01_12 {
        () => {
            set_target_pixel(dest_x + 1, dest_y, interp1(w[5], w[6]));
        };
    }
    macro_rules! pixel01_20 {
        () => {
            set_target_pixel(dest_x + 1, dest_y, interp2(w[5], w[2], w[6]));
        };
    }
    macro_rules! pixel01_21 {
        () => {
            set_target_pixel(dest_x + 1, dest_y, interp2(w[5], w[3], w[6]));
        };
    }
    macro_rules! pixel01_22 {
        () => {
            set_target_pixel(dest_x + 1, dest_y, interp2(w[5], w[3], w[2]));
        };
    }
    macro_rules! pixel01_60 {
        () => {
            set_target_pixel(dest_x + 1, dest_y, interp6(w[5], w[6], w[2]));
        };
    }
    macro_rules! pixel01_61 {
        () => {
            set_target_pixel(dest_x + 1, dest_y, interp6(w[5], w[2], w[6]));
        };
    }
    macro_rules! pixel01_70 {
        () => {
            set_target_pixel(dest_x + 1, dest_y, interp7(w[5], w[2], w[6]));
        };
    }
    macro_rules! pixel01_90 {
        () => {
            set_target_pixel(dest_x + 1, dest_y, interp9(w[5], w[2], w[6]));
        };
    }
    macro_rules! pixel01_100 {
        () => {
            set_target_pixel(dest_x + 1, dest_y, interp10(w[5], w[2], w[6]));
        };
    }

    macro_rules! pixel10_0 {
        () => {
            set_target_pixel(dest_x, dest_y + 1, w[5]);
        };
    }
    macro_rules! pixel10_10 {
        () => {
            set_target_pixel(dest_x, dest_y + 1, interp1(w[5], w[7]));
        };
    }
    macro_rules! pixel10_11 {
        () => {
            set_target_pixel(dest_x, dest_y + 1, interp1(w[5], w[8]));
        };
    }
    macro_rules! pixel10_12 {
        () => {
            set_target_pixel(dest_x, dest_y + 1, interp1(w[5], w[4]));
        };
    }
    macro_rules! pixel10_20 {
        () => {
            set_target_pixel(dest_x, dest_y + 1, interp2(w[5], w[8], w[4]));
        };
    }
    macro_rules! pixel10_21 {
        () => {
            set_target_pixel(dest_x, dest_y + 1, interp2(w[5], w[7], w[4]));
        };
    }
    macro_rules! pixel10_22 {
        () => {
            set_target_pixel(dest_x, dest_y + 1, interp2(w[5], w[7], w[8]));
        };
    }
    macro_rules! pixel10_60 {
        () => {
            set_target_pixel(dest_x, dest_y + 1, interp6(w[5], w[4], w[8]));
        };
    }
    macro_rules! pixel10_61 {
        () => {
            set_target_pixel(dest_x, dest_y + 1, interp6(w[5], w[8], w[4]));
        };
    }
    macro_rules! pixel10_70 {
        () => {
            set_target_pixel(dest_x, dest_y + 1, interp7(w[5], w[8], w[4]));
        };
    }
    macro_rules! pixel10_90 {
        () => {
            set_target_pixel(dest_x, dest_y + 1, interp9(w[5], w[8], w[4]));
        };
    }
    macro_rules! pixel10_100 {
        () => {
            set_target_pixel(dest_x, dest_y + 1, interp10(w[5], w[8], w[4]));
        };
    }

    macro_rules! pixel11_0 {
        () => {
            set_target_pixel(dest_x + 1, dest_y + 1, w[5]);
        };
    }
    macro_rules! pixel11_10 {
        () => {
            set_target_pixel(dest_x + 1, dest_y + 1, interp1(w[5], w[9]));
        };
    }
    macro_rules! pixel11_11 {
        () => {
            set_target_pixel(dest_x + 1, dest_y + 1, interp1(w[5], w[6]));
        };
    }
    macro_rules! pixel11_12 {
        () => {
            set_target_pixel(dest_x + 1, dest_y + 1, interp1(w[5], w[8]));
        };
    }
    macro_rules! pixel11_20 {
        () => {
            set_target_pixel(dest_x + 1, dest_y + 1, interp2(w[5], w[6], w[8]));
        };
    }
    macro_rules! pixel11_21 {
        () => {
            set_target_pixel(dest_x + 1, dest_y + 1, interp2(w[5], w[9], w[8]));
        };
    }
    macro_rules! pixel11_22 {
        () => {
            set_target_pixel(dest_x + 1, dest_y + 1, interp2(w[5], w[9], w[6]));
        };
    }
    macro_rules! pixel11_60 {
        () => {
            set_target_pixel(dest_x + 1, dest_y + 1, interp6(w[5], w[8], w[6]));
        };
    }
    macro_rules! pixel11_61 {
        () => {
            set_target_pixel(dest_x + 1, dest_y + 1, interp6(w[5], w[6], w[8]));
        };
    }
    macro_rules! pixel11_70 {
        () => {
            set_target_pixel(dest_x + 1, dest_y + 1, interp7(w[5], w[6], w[8]));
        };
    }
    macro_rules! pixel11_90 {
        () => {
            set_target_pixel(dest_x + 1, dest_y + 1, interp9(w[5], w[6], w[8]));
        };
    }
    macro_rules! pixel11_100 {
        () => {
            set_target_pixel(dest_x + 1, dest_y + 1, interp10(w[5], w[6], w[8]));
        };
    }

    match pattern {
        0 | 1 | 4 | 32 | 128 | 5 | 132 | 160 | 33 | 129 | 36 | 133 | 164 | 161 | 37 | 165 => {
            pixel00_20!();
            pixel01_20!();
            pixel10_20!();
            pixel11_20!();
        }
        2 | 34 | 130 | 162 => {
            pixel00_22!();
            pixel01_21!();
            pixel10_20!();
            pixel11_20!();
        }
        16 | 17 | 48 | 49 => {
            pixel00_20!();
            pixel01_22!();
            pixel10_20!();
            pixel11_21!();
        }
        64 | 65 | 68 | 69 => {
            pixel00_20!();
            pixel01_20!();
            pixel10_21!();
            pixel11_22!();
        }
        8 | 12 | 136 | 140 => {
            pixel00_21!();
            pixel01_20!();
            pixel10_22!();
            pixel11_20!();
        }
        3 | 35 | 131 | 163 => {
            pixel00_11!();
            pixel01_21!();
            pixel10_20!();
            pixel11_20!();
        }
        6 | 38 | 134 | 166 => {
            pixel00_22!();
            pixel01_12!();
            pixel10_20!();
            pixel11_20!();
        }
        20 | 21 | 52 | 53 => {
            pixel00_20!();
            pixel01_11!();
            pixel10_20!();
            pixel11_21!();
        }
        144 | 145 | 176 | 177 => {
            pixel00_20!();
            pixel01_22!();
            pixel10_20!();
            pixel11_12!();
        }
        192 | 193 | 196 | 197 => {
            pixel00_20!();
            pixel01_20!();
            pixel10_21!();
            pixel11_11!();
        }
        96 | 97 | 100 | 101 => {
            pixel00_20!();
            pixel01_20!();
            pixel10_12!();
            pixel11_22!();
        }
        40 | 44 | 168 | 172 => {
            pixel00_21!();
            pixel01_20!();
            pixel10_11!();
            pixel11_20!();
        }
        9 | 13 | 137 | 141 => {
            pixel00_12!();
            pixel01_20!();
            pixel10_22!();
            pixel11_20!();
        }
        18 | 50 => {
            pixel00_22!();
            if color_diff(w[2], w[6]) {
                pixel01_10!();
            } else {
                pixel01_20!();
            }
            pixel10_20!();
            pixel11_21!();
        }
        80 | 81 => {
            pixel00_20!();
            pixel01_22!();
            pixel10_21!();
            if color_diff(w[6], w[8]) {
                pixel11_10!();
            } else {
                pixel11_20!();
            }
        }
        72 | 76 => {
            pixel00_21!();
            pixel01_20!();
            if color_diff(w[8], w[4]) {
                pixel10_10!();
            } else {
                pixel10_20!();
            }
            pixel11_22!();
        }
        10 | 138 => {
            if color_diff(w[4], w[2]) {
                pixel00_10!();
            } else {
                pixel00_20!();
            }
            pixel01_21!();
            pixel10_22!();
            pixel11_20!();
        }
        66 => {
            pixel00_22!();
            pixel01_21!();
            pixel10_21!();
            pixel11_22!();
        }
        24 => {
            pixel00_21!();
            pixel01_22!();
            pixel10_22!();
            pixel11_21!();
        }
        7 | 39 | 135 => {
            pixel00_11!();
            pixel01_12!();
            pixel10_20!();
            pixel11_20!();
        }
        148 | 149 | 180 => {
            pixel00_20!();
            pixel01_11!();
            pixel10_20!();
            pixel11_12!();
        }
        224 | 228 | 225 => {
            pixel00_20!();
            pixel01_20!();
            pixel10_12!();
            pixel11_11!();
        }
        41 | 169 | 45 => {
            pixel00_12!();
            pixel01_20!();
            pixel10_11!();
            pixel11_20!();
        }
        22 | 54 => {
            pixel00_22!();
            if color_diff(w[2], w[6]) {
                pixel01_0!();
            } else {
                pixel01_20!();
            }
            pixel10_20!();
            pixel11_21!();
        }
        208 | 209 => {
            pixel00_20!();
            pixel01_22!();
            pixel10_21!();
            if color_diff(w[6], w[8]) {
                pixel11_0!();
            } else {
                pixel11_20!();
            }
        }
        104 | 108 => {
            pixel00_21!();
            pixel01_20!();
            if color_diff(w[8], w[4]) {
                pixel10_0!();
            } else {
                pixel10_20!();
            }
            pixel11_22!();
        }
        11 | 139 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
            } else {
                pixel00_20!();
            }
            pixel01_21!();
            pixel10_22!();
            pixel11_20!();
        }
        19 | 51 => {
            if color_diff(w[2], w[6]) {
                pixel00_11!();
                pixel01_10!();
            } else {
                pixel00_60!();
                pixel01_90!();
            }
            pixel10_20!();
            pixel11_21!();
        }
        146 | 178 => {
            pixel00_22!();
            if color_diff(w[2], w[6]) {
                pixel01_10!();
                pixel11_12!();
            } else {
                pixel01_90!();
                pixel11_61!();
            }
            pixel10_20!();
        }
        84 | 85 => {
            pixel00_20!();
            if color_diff(w[6], w[8]) {
                pixel01_11!();
                pixel11_10!();
            } else {
                pixel01_60!();
                pixel11_90!();
            }
            pixel10_21!();
        }
        112 | 113 => {
            pixel00_20!();
            pixel01_22!();
            if color_diff(w[6], w[8]) {
                pixel10_12!();
                pixel11_10!();
            } else {
                pixel10_61!();
                pixel11_90!();
            }
        }
        200 | 204 => {
            pixel00_21!();
            pixel01_20!();
            if color_diff(w[8], w[4]) {
                pixel10_10!();
                pixel11_11!();
            } else {
                pixel10_90!();
                pixel11_60!();
            }
        }
        73 | 77 => {
            if color_diff(w[8], w[4]) {
                pixel00_12!();
                pixel10_10!();
            } else {
                pixel00_61!();
                pixel10_90!();
            }
            pixel01_20!();
            pixel11_22!();
        }
        42 | 170 => {
            if color_diff(w[4], w[2]) {
                pixel00_10!();
                pixel10_11!();
            } else {
                pixel00_90!();
                pixel10_60!();
            }
            pixel01_21!();
            pixel11_20!();
        }
        14 | 142 => {
            if color_diff(w[4], w[2]) {
                pixel00_10!();
                pixel01_12!();
            } else {
                pixel00_90!();
                pixel01_61!();
            }
            pixel10_22!();
            pixel11_20!();
        }
        67 => {
            pixel00_11!();
            pixel01_21!();
            pixel10_21!();
            pixel11_22!();
        }
        70 => {
            pixel00_22!();
            pixel01_12!();
            pixel10_21!();
            pixel11_22!();
        }
        28 => {
            pixel00_21!();
            pixel01_11!();
            pixel10_22!();
            pixel11_21!();
        }
        152 => {
            pixel00_21!();
            pixel01_22!();
            pixel10_22!();
            pixel11_12!();
        }
        194 => {
            pixel00_22!();
            pixel01_21!();
            pixel10_21!();
            pixel11_11!();
        }
        98 => {
            pixel00_22!();
            pixel01_21!();
            pixel10_12!();
            pixel11_22!();
        }
        56 => {
            pixel00_21!();
            pixel01_22!();
            pixel10_11!();
            pixel11_21!();
        }
        25 => {
            pixel00_12!();
            pixel01_22!();
            pixel10_22!();
            pixel11_21!();
        }
        26 | 31 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
            } else {
                pixel00_20!();
            }
            if color_diff(w[2], w[6]) {
                pixel01_0!();
            } else {
                pixel01_20!();
            }
            pixel10_22!();
            pixel11_21!();
        }
        82 | 214 => {
            pixel00_22!();
            if color_diff(w[2], w[6]) {
                pixel01_0!();
            } else {
                pixel01_20!();
            }
            pixel10_21!();
            if color_diff(w[6], w[8]) {
                pixel11_0!();
            } else {
                pixel11_20!();
            }
        }
        88 | 248 => {
            pixel00_21!();
            pixel01_22!();
            if color_diff(w[8], w[4]) {
                pixel10_0!();
            } else {
                pixel10_20!();
            }
            if color_diff(w[6], w[8]) {
                pixel11_0!();
            } else {
                pixel11_20!();
            }
        }
        74 | 107 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
            } else {
                pixel00_20!();
            }
            pixel01_21!();
            if color_diff(w[8], w[4]) {
                pixel10_0!();
            } else {
                pixel10_20!();
            }
            pixel11_22!();
        }
        27 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
            } else {
                pixel00_20!();
            }
            pixel01_10!();
            pixel10_22!();
            pixel11_21!();
        }
        86 => {
            pixel00_22!();
            if color_diff(w[2], w[6]) {
                pixel01_0!();
            } else {
                pixel01_20!();
            }
            pixel10_21!();
            pixel11_10!();
        }
        216 => {
            pixel00_21!();
            pixel01_22!();
            pixel10_10!();
            if color_diff(w[6], w[8]) {
                pixel11_0!();
            } else {
                pixel11_20!();
            }
        }
        106 => {
            pixel00_10!();
            pixel01_21!();
            if color_diff(w[8], w[4]) {
                pixel10_0!();
            } else {
                pixel10_20!();
            }
            pixel11_22!();
        }
        30 => {
            pixel00_10!();
            if color_diff(w[2], w[6]) {
                pixel01_0!();
            } else {
                pixel01_20!();
            }
            pixel10_22!();
            pixel11_21!();
        }
        210 => {
            pixel00_22!();
            pixel01_10!();
            pixel10_21!();
            if color_diff(w[6], w[8]) {
                pixel11_0!();
            } else {
                pixel11_20!();
            }
        }
        120 => {
            pixel00_21!();
            pixel01_22!();
            if color_diff(w[8], w[4]) {
                pixel10_0!();
            } else {
                pixel10_20!();
            }
            pixel11_10!();
        }
        75 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
            } else {
                pixel00_20!();
            }
            pixel01_21!();
            pixel10_10!();
            pixel11_22!();
        }
        29 => {
            pixel00_12!();
            pixel01_11!();
            pixel10_22!();
            pixel11_21!();
        }
        198 => {
            pixel00_22!();
            pixel01_12!();
            pixel10_21!();
            pixel11_11!();
        }
        184 => {
            pixel00_21!();
            pixel01_22!();
            pixel10_11!();
            pixel11_12!();
        }
        99 => {
            pixel00_11!();
            pixel01_21!();
            pixel10_12!();
            pixel11_22!();
        }
        57 => {
            pixel00_12!();
            pixel01_22!();
            pixel10_11!();
            pixel11_21!();
        }
        71 => {
            pixel00_11!();
            pixel01_12!();
            pixel10_21!();
            pixel11_22!();
        }
        156 => {
            pixel00_21!();
            pixel01_11!();
            pixel10_22!();
            pixel11_12!();
        }
        226 => {
            pixel00_22!();
            pixel01_21!();
            pixel10_12!();
            pixel11_11!();
        }
        60 => {
            pixel00_21!();
            pixel01_11!();
            pixel10_11!();
            pixel11_21!();
        }
        195 => {
            pixel00_11!();
            pixel01_21!();
            pixel10_21!();
            pixel11_11!();
        }
        102 => {
            pixel00_22!();
            pixel01_12!();
            pixel10_12!();
            pixel11_22!();
        }
        153 => {
            pixel00_12!();
            pixel01_22!();
            pixel10_22!();
            pixel11_12!();
        }
        58 => {
            if color_diff(w[4], w[2]) {
                pixel00_10!();
            } else {
                pixel00_70!();
            }
            if color_diff(w[2], w[6]) {
                pixel01_10!();
            } else {
                pixel01_70!();
            }
            pixel10_11!();
            pixel11_21!();
        }
        83 => {
            pixel00_11!();
            if color_diff(w[2], w[6]) {
                pixel01_10!();
            } else {
                pixel01_70!();
            }
            pixel10_21!();
            if color_diff(w[6], w[8]) {
                pixel11_10!();
            } else {
                pixel11_70!();
            }
        }
        92 => {
            pixel00_21!();
            pixel01_11!();
            if color_diff(w[8], w[4]) {
                pixel10_10!();
            } else {
                pixel10_70!();
            }
            if color_diff(w[6], w[8]) {
                pixel11_10!();
            } else {
                pixel11_70!();
            }
        }
        202 => {
            if color_diff(w[4], w[2]) {
                pixel00_10!();
            } else {
                pixel00_70!();
            }
            pixel01_21!();
            if color_diff(w[8], w[4]) {
                pixel10_10!();
            } else {
                pixel10_70!();
            }
            pixel11_11!();
        }
        78 => {
            if color_diff(w[4], w[2]) {
                pixel00_10!();
            } else {
                pixel00_70!();
            }
            pixel01_12!();
            if color_diff(w[8], w[4]) {
                pixel10_10!();
            } else {
                pixel10_70!();
            }
            pixel11_22!();
        }
        154 => {
            if color_diff(w[4], w[2]) {
                pixel00_10!();
            } else {
                pixel00_70!();
            }
            if color_diff(w[2], w[6]) {
                pixel01_10!();
            } else {
                pixel01_70!();
            }
            pixel10_22!();
            pixel11_12!();
        }
        114 => {
            pixel00_22!();
            if color_diff(w[2], w[6]) {
                pixel01_10!();
            } else {
                pixel01_70!();
            }
            pixel10_12!();
            if color_diff(w[6], w[8]) {
                pixel11_10!();
            } else {
                pixel11_70!();
            }
        }
        89 => {
            pixel00_12!();
            pixel01_22!();
            if color_diff(w[8], w[4]) {
                pixel10_10!();
            } else {
                pixel10_70!();
            }
            if color_diff(w[6], w[8]) {
                pixel11_10!();
            } else {
                pixel11_70!();
            }
        }
        90 => {
            if color_diff(w[4], w[2]) {
                pixel00_10!();
            } else {
                pixel00_70!();
            }
            if color_diff(w[2], w[6]) {
                pixel01_10!();
            } else {
                pixel01_70!();
            }
            if color_diff(w[8], w[4]) {
                pixel10_10!();
            } else {
                pixel10_70!();
            }
            if color_diff(w[6], w[8]) {
                pixel11_10!();
            } else {
                pixel11_70!();
            }
        }
        55 | 23 => {
            if color_diff(w[2], w[6]) {
                pixel00_11!();
                pixel01_0!();
            } else {
                pixel00_60!();
                pixel01_90!();
            }
            pixel10_20!();
            pixel11_21!();
        }
        182 | 150 => {
            pixel00_22!();
            if color_diff(w[2], w[6]) {
                pixel01_0!();
                pixel11_12!();
            } else {
                pixel01_90!();
                pixel11_61!();
            }
            pixel10_20!();
        }
        213 | 212 => {
            pixel00_20!();
            if color_diff(w[6], w[8]) {
                pixel01_11!();
                pixel11_0!();
            } else {
                pixel01_60!();
                pixel11_90!();
            }
            pixel10_21!();
        }
        241 | 240 => {
            pixel00_20!();
            pixel01_22!();
            if color_diff(w[6], w[8]) {
                pixel10_12!();
                pixel11_0!();
            } else {
                pixel10_61!();
                pixel11_90!();
            }
        }
        236 | 232 => {
            pixel00_21!();
            pixel01_20!();
            if color_diff(w[8], w[4]) {
                pixel10_0!();
                pixel11_11!();
            } else {
                pixel10_90!();
                pixel11_60!();
            }
        }
        109 | 105 => {
            if color_diff(w[8], w[4]) {
                pixel00_12!();
                pixel10_0!();
            } else {
                pixel00_61!();
                pixel10_90!();
            }
            pixel01_20!();
            pixel11_22!();
        }
        171 | 43 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
                pixel10_11!();
            } else {
                pixel00_90!();
                pixel10_60!();
            }
            pixel01_21!();
            pixel11_20!();
        }
        143 | 15 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
                pixel01_12!();
            } else {
                pixel00_90!();
                pixel01_61!();
            }
            pixel10_22!();
            pixel11_20!();
        }
        124 => {
            pixel00_21!();
            pixel01_11!();
            if color_diff(w[8], w[4]) {
                pixel10_0!();
            } else {
                pixel10_20!();
            }
            pixel11_10!();
        }
        203 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
            } else {
                pixel00_20!();
            }
            pixel01_21!();
            pixel10_10!();
            pixel11_11!();
        }
        62 => {
            pixel00_10!();
            if color_diff(w[2], w[6]) {
                pixel01_0!();
            } else {
                pixel01_20!();
            }
            pixel10_11!();
            pixel11_21!();
        }
        211 => {
            pixel00_11!();
            pixel01_10!();
            pixel10_21!();
            if color_diff(w[6], w[8]) {
                pixel11_0!();
            } else {
                pixel11_20!();
            }
        }
        118 => {
            pixel00_22!();
            if color_diff(w[2], w[6]) {
                pixel01_0!();
            } else {
                pixel01_20!();
            }
            pixel10_12!();
            pixel11_10!();
        }
        217 => {
            pixel00_12!();
            pixel01_22!();
            pixel10_10!();
            if color_diff(w[6], w[8]) {
                pixel11_0!();
            } else {
                pixel11_20!();
            }
        }
        110 => {
            pixel00_10!();
            pixel01_12!();
            if color_diff(w[8], w[4]) {
                pixel10_0!();
            } else {
                pixel10_20!();
            }
            pixel11_22!();
        }
        155 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
            } else {
                pixel00_20!();
            }
            pixel01_10!();
            pixel10_22!();
            pixel11_12!();
        }
        188 => {
            pixel00_21!();
            pixel01_11!();
            pixel10_11!();
            pixel11_12!();
        }
        185 => {
            pixel00_12!();
            pixel01_22!();
            pixel10_11!();
            pixel11_12!();
        }
        61 => {
            pixel00_12!();
            pixel01_11!();
            pixel10_11!();
            pixel11_21!();
        }
        157 => {
            pixel00_12!();
            pixel01_11!();
            pixel10_22!();
            pixel11_12!();
        }
        103 => {
            pixel00_11!();
            pixel01_12!();
            pixel10_12!();
            pixel11_22!();
        }
        227 => {
            pixel00_11!();
            pixel01_21!();
            pixel10_12!();
            pixel11_11!();
        }
        230 => {
            pixel00_22!();
            pixel01_12!();
            pixel10_12!();
            pixel11_11!();
        }
        199 => {
            pixel00_11!();
            pixel01_12!();
            pixel10_21!();
            pixel11_11!();
        }
        220 => {
            pixel00_21!();
            pixel01_11!();
            if color_diff(w[8], w[4]) {
                pixel10_10!();
            } else {
                pixel10_70!();
            }
            if color_diff(w[6], w[8]) {
                pixel11_0!();
            } else {
                pixel11_20!();
            }
        }
        158 => {
            if color_diff(w[4], w[2]) {
                pixel00_10!();
            } else {
                pixel00_70!();
            }
            if color_diff(w[2], w[6]) {
                pixel01_0!();
            } else {
                pixel01_20!();
            }
            pixel10_22!();
            pixel11_12!();
        }
        234 => {
            if color_diff(w[4], w[2]) {
                pixel00_10!();
            } else {
                pixel00_70!();
            }
            pixel01_21!();
            if color_diff(w[8], w[4]) {
                pixel10_0!();
            } else {
                pixel10_20!();
            }
            pixel11_11!();
        }
        242 => {
            pixel00_22!();
            if color_diff(w[2], w[6]) {
                pixel01_10!();
            } else {
                pixel01_70!();
            }
            pixel10_12!();
            if color_diff(w[6], w[8]) {
                pixel11_0!();
            } else {
                pixel11_20!();
            }
        }
        59 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
            } else {
                pixel00_20!();
            }
            if color_diff(w[2], w[6]) {
                pixel01_10!();
            } else {
                pixel01_70!();
            }
            pixel10_11!();
            pixel11_21!();
        }
        121 => {
            pixel00_12!();
            pixel01_22!();
            if color_diff(w[8], w[4]) {
                pixel10_0!();
            } else {
                pixel10_20!();
            }
            if color_diff(w[6], w[8]) {
                pixel11_10!();
            } else {
                pixel11_70!();
            }
        }
        87 => {
            pixel00_11!();
            if color_diff(w[2], w[6]) {
                pixel01_0!();
            } else {
                pixel01_20!();
            }
            pixel10_21!();
            if color_diff(w[6], w[8]) {
                pixel11_10!();
            } else {
                pixel11_70!();
            }
        }
        79 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
            } else {
                pixel00_20!();
            }
            pixel01_12!();
            if color_diff(w[8], w[4]) {
                pixel10_10!();
            } else {
                pixel10_70!();
            }
            pixel11_22!();
        }
        122 => {
            if color_diff(w[4], w[2]) {
                pixel00_10!();
            } else {
                pixel00_70!();
            }
            if color_diff(w[2], w[6]) {
                pixel01_10!();
            } else {
                pixel01_70!();
            }
            if color_diff(w[8], w[4]) {
                pixel10_0!();
            } else {
                pixel10_20!();
            }
            if color_diff(w[6], w[8]) {
                pixel11_10!();
            } else {
                pixel11_70!();
            }
        }
        94 => {
            if color_diff(w[4], w[2]) {
                pixel00_10!();
            } else {
                pixel00_70!();
            }
            if color_diff(w[2], w[6]) {
                pixel01_0!();
            } else {
                pixel01_20!();
            }
            if color_diff(w[8], w[4]) {
                pixel10_10!();
            } else {
                pixel10_70!();
            }
            if color_diff(w[6], w[8]) {
                pixel11_10!();
            } else {
                pixel11_70!();
            }
        }
        218 => {
            if color_diff(w[4], w[2]) {
                pixel00_10!();
            } else {
                pixel00_70!();
            }
            if color_diff(w[2], w[6]) {
                pixel01_10!();
            } else {
                pixel01_70!();
            }
            if color_diff(w[8], w[4]) {
                pixel10_10!();
            } else {
                pixel10_70!();
            }
            if color_diff(w[6], w[8]) {
                pixel11_0!();
            } else {
                pixel11_20!();
            }
        }
        91 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
            } else {
                pixel00_20!();
            }
            if color_diff(w[2], w[6]) {
                pixel01_10!();
            } else {
                pixel01_70!();
            }
            if color_diff(w[8], w[4]) {
                pixel10_10!();
            } else {
                pixel10_70!();
            }
            if color_diff(w[6], w[8]) {
                pixel11_10!();
            } else {
                pixel11_70!();
            }
        }
        229 => {
            pixel00_20!();
            pixel01_20!();
            pixel10_12!();
            pixel11_11!();
        }
        167 => {
            pixel00_11!();
            pixel01_12!();
            pixel10_20!();
            pixel11_20!();
        }
        173 => {
            pixel00_12!();
            pixel01_20!();
            pixel10_11!();
            pixel11_20!();
        }
        181 => {
            pixel00_20!();
            pixel01_11!();
            pixel10_20!();
            pixel11_12!();
        }
        186 => {
            if color_diff(w[4], w[2]) {
                pixel00_10!();
            } else {
                pixel00_70!();
            }
            if color_diff(w[2], w[6]) {
                pixel01_10!();
            } else {
                pixel01_70!();
            }
            pixel10_11!();
            pixel11_12!();
        }
        115 => {
            pixel00_11!();
            if color_diff(w[2], w[6]) {
                pixel01_10!();
            } else {
                pixel01_70!();
            }
            pixel10_12!();
            if color_diff(w[6], w[8]) {
                pixel11_10!();
            } else {
                pixel11_70!();
            }
        }
        93 => {
            pixel00_12!();
            pixel01_11!();
            if color_diff(w[8], w[4]) {
                pixel10_10!();
            } else {
                pixel10_70!();
            }
            if color_diff(w[6], w[8]) {
                pixel11_10!();
            } else {
                pixel11_70!();
            }
        }
        206 => {
            if color_diff(w[4], w[2]) {
                pixel00_10!();
            } else {
                pixel00_70!();
            }
            pixel01_12!();
            if color_diff(w[8], w[4]) {
                pixel10_10!();
            } else {
                pixel10_70!();
            }
            pixel11_11!();
        }
        205 | 201 => {
            pixel00_12!();
            pixel01_20!();
            if color_diff(w[8], w[4]) {
                pixel10_10!();
            } else {
                pixel10_70!();
            }
            pixel11_11!();
        }
        174 | 46 => {
            if color_diff(w[4], w[2]) {
                pixel00_10!();
            } else {
                pixel00_70!();
            }
            pixel01_12!();
            pixel10_11!();
            pixel11_20!();
        }
        179 | 147 => {
            pixel00_11!();
            if color_diff(w[2], w[6]) {
                pixel01_10!();
            } else {
                pixel01_70!();
            }
            pixel10_20!();
            pixel11_12!();
        }
        117 | 116 => {
            pixel00_20!();
            pixel01_11!();
            pixel10_12!();
            if color_diff(w[6], w[8]) {
                pixel11_10!();
            } else {
                pixel11_70!();
            }
        }
        189 => {
            pixel00_12!();
            pixel01_11!();
            pixel10_11!();
            pixel11_12!();
        }
        231 => {
            pixel00_11!();
            pixel01_12!();
            pixel10_12!();
            pixel11_11!();
        }
        126 => {
            pixel00_10!();
            if color_diff(w[2], w[6]) {
                pixel01_0!();
            } else {
                pixel01_20!();
            }
            if color_diff(w[8], w[4]) {
                pixel10_0!();
            } else {
                pixel10_20!();
            }
            pixel11_10!();
        }
        219 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
            } else {
                pixel00_20!();
            }
            pixel01_10!();
            pixel10_10!();
            if color_diff(w[6], w[8]) {
                pixel11_0!();
            } else {
                pixel11_20!();
            }
        }
        125 => {
            if color_diff(w[8], w[4]) {
                pixel00_12!();
                pixel10_0!();
            } else {
                pixel00_61!();
                pixel10_90!();
            }
            pixel01_11!();
            pixel11_10!();
        }
        221 => {
            pixel00_12!();
            if color_diff(w[6], w[8]) {
                pixel01_11!();
                pixel11_0!();
            } else {
                pixel01_60!();
                pixel11_90!();
            }
            pixel10_10!();
        }
        207 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
                pixel01_12!();
            } else {
                pixel00_90!();
                pixel01_61!();
            }
            pixel10_10!();
            pixel11_11!();
        }
        238 => {
            pixel00_10!();
            pixel01_12!();
            if color_diff(w[8], w[4]) {
                pixel10_0!();
                pixel11_11!();
            } else {
                pixel10_90!();
                pixel11_60!();
            }
        }
        190 => {
            pixel00_10!();
            if color_diff(w[2], w[6]) {
                pixel01_0!();
                pixel11_12!();
            } else {
                pixel01_90!();
                pixel11_61!();
            }
            pixel10_11!();
        }
        187 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
                pixel10_11!();
            } else {
                pixel00_90!();
                pixel10_60!();
            }
            pixel01_10!();
            pixel11_12!();
        }
        243 => {
            pixel00_11!();
            pixel01_10!();
            if color_diff(w[6], w[8]) {
                pixel10_12!();
                pixel11_0!();
            } else {
                pixel10_61!();
                pixel11_90!();
            }
        }
        119 => {
            if color_diff(w[2], w[6]) {
                pixel00_11!();
                pixel01_0!();
            } else {
                pixel00_60!();
                pixel01_90!();
            }
            pixel10_12!();
            pixel11_10!();
        }
        237 | 233 => {
            pixel00_12!();
            pixel01_20!();
            if color_diff(w[8], w[4]) {
                pixel10_0!();
            } else {
                pixel10_100!();
            }
            pixel11_11!();
        }
        175 | 47 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
            } else {
                pixel00_100!();
            }
            pixel01_12!();
            pixel10_11!();
            pixel11_20!();
        }
        183 | 151 => {
            pixel00_11!();
            if color_diff(w[2], w[6]) {
                pixel01_0!();
            } else {
                pixel01_100!();
            }
            pixel10_20!();
            pixel11_12!();
        }
        245 | 244 => {
            pixel00_20!();
            pixel01_11!();
            pixel10_12!();
            if color_diff(w[6], w[8]) {
                pixel11_0!();
            } else {
                pixel11_100!();
            }
        }
        250 => {
            pixel00_10!();
            pixel01_10!();
            if color_diff(w[8], w[4]) {
                pixel10_0!();
            } else {
                pixel10_20!();
            }
            if color_diff(w[6], w[8]) {
                pixel11_0!();
            } else {
                pixel11_20!();
            }
        }
        123 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
            } else {
                pixel00_20!();
            }
            pixel01_10!();
            if color_diff(w[8], w[4]) {
                pixel10_0!();
            } else {
                pixel10_20!();
            }
            pixel11_10!();
        }
        95 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
            } else {
                pixel00_20!();
            }
            if color_diff(w[2], w[6]) {
                pixel01_0!();
            } else {
                pixel01_20!();
            }
            pixel10_10!();
            pixel11_10!();
        }
        222 => {
            pixel00_10!();
            if color_diff(w[2], w[6]) {
                pixel01_0!();
            } else {
                pixel01_20!();
            }
            pixel10_10!();
            if color_diff(w[6], w[8]) {
                pixel11_0!();
            } else {
                pixel11_20!();
            }
        }
        252 => {
            pixel00_21!();
            pixel01_11!();
            if color_diff(w[8], w[4]) {
                pixel10_0!();
            } else {
                pixel10_20!();
            }
            if color_diff(w[6], w[8]) {
                pixel11_0!();
            } else {
                pixel11_100!();
            }
        }
        249 => {
            pixel00_12!();
            pixel01_22!();
            if color_diff(w[8], w[4]) {
                pixel10_0!();
            } else {
                pixel10_100!();
            }
            if color_diff(w[6], w[8]) {
                pixel11_0!();
            } else {
                pixel11_20!();
            }
        }
        235 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
            } else {
                pixel00_20!();
            }
            pixel01_21!();
            if color_diff(w[8], w[4]) {
                pixel10_0!();
            } else {
                pixel10_100!();
            }
            pixel11_11!();
        }
        111 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
            } else {
                pixel00_100!();
            }
            pixel01_12!();
            if color_diff(w[8], w[4]) {
                pixel10_0!();
            } else {
                pixel10_20!();
            }
            pixel11_22!();
        }
        63 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
            } else {
                pixel00_100!();
            }
            if color_diff(w[2], w[6]) {
                pixel01_0!();
            } else {
                pixel01_20!();
            }
            pixel10_11!();
            pixel11_21!();
        }
        159 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
            } else {
                pixel00_20!();
            }
            if color_diff(w[2], w[6]) {
                pixel01_0!();
            } else {
                pixel01_100!();
            }
            pixel10_22!();
            pixel11_12!();
        }
        215 => {
            pixel00_11!();
            if color_diff(w[2], w[6]) {
                pixel01_0!();
            } else {
                pixel01_100!();
            }
            pixel10_21!();
            if color_diff(w[6], w[8]) {
                pixel11_0!();
            } else {
                pixel11_20!();
            }
        }
        246 => {
            pixel00_22!();
            if color_diff(w[2], w[6]) {
                pixel01_0!();
            } else {
                pixel01_20!();
            }
            pixel10_12!();
            if color_diff(w[6], w[8]) {
                pixel11_0!();
            } else {
                pixel11_100!();
            }
        }
        254 => {
            pixel00_10!();
            if color_diff(w[2], w[6]) {
                pixel01_0!();
            } else {
                pixel01_20!();
            }
            if color_diff(w[8], w[4]) {
                pixel10_0!();
            } else {
                pixel10_20!();
            }
            if color_diff(w[6], w[8]) {
                pixel11_0!();
            } else {
                pixel11_100!();
            }
        }
        253 => {
            pixel00_12!();
            pixel01_11!();
            if color_diff(w[8], w[4]) {
                pixel10_0!();
            } else {
                pixel10_100!();
            }
            if color_diff(w[6], w[8]) {
                pixel11_0!();
            } else {
                pixel11_100!();
            }
        }
        251 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
            } else {
                pixel00_20!();
            }
            pixel01_10!();
            if color_diff(w[8], w[4]) {
                pixel10_0!();
            } else {
                pixel10_100!();
            }
            if color_diff(w[6], w[8]) {
                pixel11_0!();
            } else {
                pixel11_20!();
            }
        }
        239 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
            } else {
                pixel00_100!();
            }
            pixel01_12!();
            if color_diff(w[8], w[4]) {
                pixel10_0!();
            } else {
                pixel10_100!();
            }
            pixel11_11!();
        }
        127 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
            } else {
                pixel00_100!();
            }
            if color_diff(w[2], w[6]) {
                pixel01_0!();
            } else {
                pixel01_20!();
            }
            if color_diff(w[8], w[4]) {
                pixel10_0!();
            } else {
                pixel10_20!();
            }
            pixel11_10!();
        }
        191 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
            } else {
                pixel00_100!();
            }
            if color_diff(w[2], w[6]) {
                pixel01_0!();
            } else {
                pixel01_100!();
            }
            pixel10_11!();
            pixel11_12!();
        }
        223 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
            } else {
                pixel00_20!();
            }
            if color_diff(w[2], w[6]) {
                pixel01_0!();
            } else {
                pixel01_100!();
            }
            pixel10_10!();
            if color_diff(w[6], w[8]) {
                pixel11_0!();
            } else {
                pixel11_20!();
            }
        }
        247 => {
            pixel00_11!();
            if color_diff(w[2], w[6]) {
                pixel01_0!();
            } else {
                pixel01_100!();
            }
            pixel10_12!();
            if color_diff(w[6], w[8]) {
                pixel11_0!();
            } else {
                pixel11_100!();
            }
        }
        255 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
            } else {
                pixel00_100!();
            }
            if color_diff(w[2], w[6]) {
                pixel01_0!();
            } else {
                pixel01_100!();
            }
            if color_diff(w[8], w[4]) {
                pixel10_0!();
            } else {
                pixel10_100!();
            }
            if color_diff(w[6], w[8]) {
                pixel11_0!();
            } else {
                pixel11_100!();
            }
        }
        _ => unreachable!(),
    }
}

fn hq3x_inner(
    w: &[Color; 10],
    target_buffer: &mut [Color],
    offset: usize,
    pattern: i32,
    dest_x: usize,
    dest_y: usize,
    source_width: usize,
) {
    let mut set_target_pixel = |x: usize, y: usize, c: Color| {
        let index = (y * source_width * 3) + x;
        target_buffer[index - offset] = c;
    };

    macro_rules! pixel00_1m {
        () => {
            set_target_pixel(dest_x, dest_y, interp1(w[5], w[1]));
        };
    }
    macro_rules! pixel00_1u {
        () => {
            set_target_pixel(dest_x, dest_y, interp1(w[5], w[2]));
        };
    }
    macro_rules! pixel00_1l {
        () => {
            set_target_pixel(dest_x, dest_y, interp1(w[5], w[4]));
        };
    }
    macro_rules! pixel00_2 {
        () => {
            set_target_pixel(dest_x, dest_y, interp2(w[5], w[4], w[2]));
        };
    }
    macro_rules! pixel00_4 {
        () => {
            set_target_pixel(dest_x, dest_y, interp4(w[5], w[4], w[2]));
        };
    }
    macro_rules! pixel00_5 {
        () => {
            set_target_pixel(dest_x, dest_y, interp5(w[4], w[2]));
        };
    }
    macro_rules! pixel00_c {
        () => {
            set_target_pixel(dest_x, dest_y, w[5]);
        };
    }

    macro_rules! pixel01_1 {
        () => {
            set_target_pixel(dest_x + 1, dest_y, interp1(w[5], w[2]));
        };
    }
    macro_rules! pixel01_3 {
        () => {
            set_target_pixel(dest_x + 1, dest_y, interp3(w[5], w[2]));
        };
    }
    macro_rules! pixel01_6 {
        () => {
            set_target_pixel(dest_x + 1, dest_y, interp1(w[2], w[5]));
        };
    }
    macro_rules! pixel01_c {
        () => {
            set_target_pixel(dest_x + 1, dest_y, w[5]);
        };
    }

    macro_rules! pixel02_1m {
        () => {
            set_target_pixel(dest_x + 2, dest_y, interp1(w[5], w[3]));
        };
    }
    macro_rules! pixel02_1u {
        () => {
            set_target_pixel(dest_x + 2, dest_y, interp1(w[5], w[2]));
        };
    }
    macro_rules! pixel02_1r {
        () => {
            set_target_pixel(dest_x + 2, dest_y, interp1(w[5], w[6]));
        };
    }
    macro_rules! pixel02_2 {
        () => {
            set_target_pixel(dest_x + 2, dest_y, interp2(w[5], w[2], w[6]));
        };
    }
    macro_rules! pixel02_4 {
        () => {
            set_target_pixel(dest_x + 2, dest_y, interp4(w[5], w[2], w[6]));
        };
    }
    macro_rules! pixel02_5 {
        () => {
            set_target_pixel(dest_x + 2, dest_y, interp5(w[2], w[6]));
        };
    }
    macro_rules! pixel02_c {
        () => {
            set_target_pixel(dest_x + 2, dest_y, w[5]);
        };
    }

    macro_rules! pixel10_1 {
        () => {
            set_target_pixel(dest_x, dest_y + 1, interp1(w[5], w[4]));
        };
    }
    macro_rules! pixel10_3 {
        () => {
            set_target_pixel(dest_x, dest_y + 1, interp3(w[5], w[4]));
        };
    }
    macro_rules! pixel10_6 {
        () => {
            set_target_pixel(dest_x, dest_y + 1, interp1(w[4], w[5]));
        };
    }
    macro_rules! pixel10_c {
        () => {
            set_target_pixel(dest_x, dest_y + 1, w[5]);
        };
    }

    macro_rules! pixel11 {
        () => {
            set_target_pixel(dest_x + 1, dest_y + 1, w[5]);
        };
    }

    macro_rules! pixel12_1 {
        () => {
            set_target_pixel(dest_x + 2, dest_y + 1, interp1(w[5], w[6]));
        };
    }
    macro_rules! pixel12_3 {
        () => {
            set_target_pixel(dest_x + 2, dest_y + 1, interp3(w[5], w[6]));
        };
    }
    macro_rules! pixel12_6 {
        () => {
            set_target_pixel(dest_x + 2, dest_y + 1, interp1(w[6], w[5]));
        };
    }
    macro_rules! pixel12_c {
        () => {
            set_target_pixel(dest_x + 2, dest_y + 1, w[5]);
        };
    }

    macro_rules! pixel20_1m {
        () => {
            set_target_pixel(dest_x, dest_y + 2, interp1(w[5], w[7]));
        };
    }
    macro_rules! pixel20_1d {
        () => {
            set_target_pixel(dest_x, dest_y + 2, interp1(w[5], w[8]));
        };
    }
    macro_rules! pixel20_1l {
        () => {
            set_target_pixel(dest_x, dest_y + 2, interp1(w[5], w[4]));
        };
    }
    macro_rules! pixel20_2 {
        () => {
            set_target_pixel(dest_x, dest_y + 2, interp2(w[5], w[8], w[4]));
        };
    }
    macro_rules! pixel20_4 {
        () => {
            set_target_pixel(dest_x, dest_y + 2, interp4(w[5], w[8], w[4]));
        };
    }
    macro_rules! pixel20_5 {
        () => {
            set_target_pixel(dest_x, dest_y + 2, interp5(w[8], w[4]));
        };
    }
    macro_rules! pixel20_c {
        () => {
            set_target_pixel(dest_x, dest_y + 2, w[5]);
        };
    }

    macro_rules! pixel21_1 {
        () => {
            set_target_pixel(dest_x + 1, dest_y + 2, interp1(w[5], w[8]));
        };
    }
    macro_rules! pixel21_3 {
        () => {
            set_target_pixel(dest_x + 1, dest_y + 2, interp3(w[5], w[8]));
        };
    }
    macro_rules! pixel21_6 {
        () => {
            set_target_pixel(dest_x + 1, dest_y + 2, interp1(w[8], w[5]));
        };
    }
    macro_rules! pixel21_c {
        () => {
            set_target_pixel(dest_x + 1, dest_y + 2, w[5]);
        };
    }

    macro_rules! pixel22_1m {
        () => {
            set_target_pixel(dest_x + 2, dest_y + 2, interp1(w[5], w[9]));
        };
    }
    macro_rules! pixel22_1d {
        () => {
            set_target_pixel(dest_x + 2, dest_y + 2, interp1(w[5], w[8]));
        };
    }
    macro_rules! pixel22_1r {
        () => {
            set_target_pixel(dest_x + 2, dest_y + 2, interp1(w[5], w[6]));
        };
    }
    macro_rules! pixel22_2 {
        () => {
            set_target_pixel(dest_x + 2, dest_y + 2, interp2(w[5], w[6], w[8]));
        };
    }
    macro_rules! pixel22_4 {
        () => {
            set_target_pixel(dest_x + 2, dest_y + 2, interp4(w[5], w[6], w[8]));
        };
    }
    macro_rules! pixel22_5 {
        () => {
            set_target_pixel(dest_x + 2, dest_y + 2, interp5(w[6], w[8]));
        };
    }
    macro_rules! pixel22_c {
        () => {
            set_target_pixel(dest_x + 2, dest_y + 2, w[5]);
        };
    }

    match pattern {
        0 | 1 | 4 | 32 | 128 | 5 | 132 | 160 | 33 | 129 | 36 | 133 | 164 | 161 | 37 | 165 => {
            pixel00_2!();
            pixel01_1!();
            pixel02_2!();
            pixel10_1!();
            pixel11!();
            pixel12_1!();
            pixel20_2!();
            pixel21_1!();
            pixel22_2!();
        }
        2 | 34 | 130 | 162 => {
            pixel00_1m!();
            pixel01_c!();
            pixel02_1m!();
            pixel10_1!();
            pixel11!();
            pixel12_1!();
            pixel20_2!();
            pixel21_1!();
            pixel22_2!();
        }
        16 | 17 | 48 | 49 => {
            pixel00_2!();
            pixel01_1!();
            pixel02_1m!();
            pixel10_1!();
            pixel11!();
            pixel12_c!();
            pixel20_2!();
            pixel21_1!();
            pixel22_1m!();
        }
        64 | 65 | 68 | 69 => {
            pixel00_2!();
            pixel01_1!();
            pixel02_2!();
            pixel10_1!();
            pixel11!();
            pixel12_1!();
            pixel20_1m!();
            pixel21_c!();
            pixel22_1m!();
        }
        8 | 12 | 136 | 140 => {
            pixel00_1m!();
            pixel01_1!();
            pixel02_2!();
            pixel10_c!();
            pixel11!();
            pixel12_1!();
            pixel20_1m!();
            pixel21_1!();
            pixel22_2!();
        }
        3 | 35 | 131 | 163 => {
            pixel00_1l!();
            pixel01_c!();
            pixel02_1m!();
            pixel10_1!();
            pixel11!();
            pixel12_1!();
            pixel20_2!();
            pixel21_1!();
            pixel22_2!();
        }
        6 | 38 | 134 | 166 => {
            pixel00_1m!();
            pixel01_c!();
            pixel02_1r!();
            pixel10_1!();
            pixel11!();
            pixel12_1!();
            pixel20_2!();
            pixel21_1!();
            pixel22_2!();
        }
        20 | 21 | 52 | 53 => {
            pixel00_2!();
            pixel01_1!();
            pixel02_1u!();
            pixel10_1!();
            pixel11!();
            pixel12_c!();
            pixel20_2!();
            pixel21_1!();
            pixel22_1m!();
        }
        144 | 145 | 176 | 177 => {
            pixel00_2!();
            pixel01_1!();
            pixel02_1m!();
            pixel10_1!();
            pixel11!();
            pixel12_c!();
            pixel20_2!();
            pixel21_1!();
            pixel22_1d!();
        }
        192 | 193 | 196 | 197 => {
            pixel00_2!();
            pixel01_1!();
            pixel02_2!();
            pixel10_1!();
            pixel11!();
            pixel12_1!();
            pixel20_1m!();
            pixel21_c!();
            pixel22_1r!();
        }
        96 | 97 | 100 | 101 => {
            pixel00_2!();
            pixel01_1!();
            pixel02_2!();
            pixel10_1!();
            pixel11!();
            pixel12_1!();
            pixel20_1l!();
            pixel21_c!();
            pixel22_1m!();
        }
        40 | 44 | 168 | 172 => {
            pixel00_1m!();
            pixel01_1!();
            pixel02_2!();
            pixel10_c!();
            pixel11!();
            pixel12_1!();
            pixel20_1d!();
            pixel21_1!();
            pixel22_2!();
        }
        9 | 13 | 137 | 141 => {
            pixel00_1u!();
            pixel01_1!();
            pixel02_2!();
            pixel10_c!();
            pixel11!();
            pixel12_1!();
            pixel20_1m!();
            pixel21_1!();
            pixel22_2!();
        }
        18 | 50 => {
            pixel00_1m!();
            if color_diff(w[2], w[6]) {
                pixel01_c!();
                pixel02_1m!();
                pixel12_c!();
            } else {
                pixel01_3!();
                pixel02_4!();
                pixel12_3!();
            }
            pixel10_1!();
            pixel11!();
            pixel20_2!();
            pixel21_1!();
            pixel22_1m!();
        }
        80 | 81 => {
            pixel00_2!();
            pixel01_1!();
            pixel02_1m!();
            pixel10_1!();
            pixel11!();
            pixel20_1m!();
            if color_diff(w[6], w[8]) {
                pixel12_c!();
                pixel21_c!();
                pixel22_1m!();
            } else {
                pixel12_3!();
                pixel21_3!();
                pixel22_4!();
            }
        }
        72 | 76 => {
            pixel00_1m!();
            pixel01_1!();
            pixel02_2!();
            pixel11!();
            pixel12_1!();
            if color_diff(w[8], w[4]) {
                pixel10_c!();
                pixel20_1m!();
                pixel21_c!();
            } else {
                pixel10_3!();
                pixel20_4!();
                pixel21_3!();
            }
            pixel22_1m!();
        }
        10 | 138 => {
            if color_diff(w[4], w[2]) {
                pixel00_1m!();
                pixel01_c!();
                pixel10_c!();
            } else {
                pixel00_4!();
                pixel01_3!();
                pixel10_3!();
            }
            pixel02_1m!();
            pixel11!();
            pixel12_1!();
            pixel20_1m!();
            pixel21_1!();
            pixel22_2!();
        }
        66 => {
            pixel00_1m!();
            pixel01_c!();
            pixel02_1m!();
            pixel10_1!();
            pixel11!();
            pixel12_1!();
            pixel20_1m!();
            pixel21_c!();
            pixel22_1m!();
        }
        24 => {
            pixel00_1m!();
            pixel01_1!();
            pixel02_1m!();
            pixel10_c!();
            pixel11!();
            pixel12_c!();
            pixel20_1m!();
            pixel21_1!();
            pixel22_1m!();
        }
        7 | 39 | 135 => {
            pixel00_1l!();
            pixel01_c!();
            pixel02_1r!();
            pixel10_1!();
            pixel11!();
            pixel12_1!();
            pixel20_2!();
            pixel21_1!();
            pixel22_2!();
        }
        148 | 149 | 180 => {
            pixel00_2!();
            pixel01_1!();
            pixel02_1u!();
            pixel10_1!();
            pixel11!();
            pixel12_c!();
            pixel20_2!();
            pixel21_1!();
            pixel22_1d!();
        }
        224 | 228 | 225 => {
            pixel00_2!();
            pixel01_1!();
            pixel02_2!();
            pixel10_1!();
            pixel11!();
            pixel12_1!();
            pixel20_1l!();
            pixel21_c!();
            pixel22_1r!();
        }
        41 | 169 | 45 => {
            pixel00_1u!();
            pixel01_1!();
            pixel02_2!();
            pixel10_c!();
            pixel11!();
            pixel12_1!();
            pixel20_1d!();
            pixel21_1!();
            pixel22_2!();
        }
        22 | 54 => {
            pixel00_1m!();
            if color_diff(w[2], w[6]) {
                pixel01_c!();
                pixel02_c!();
                pixel12_c!();
            } else {
                pixel01_3!();
                pixel02_4!();
                pixel12_3!();
            }
            pixel10_1!();
            pixel11!();
            pixel20_2!();
            pixel21_1!();
            pixel22_1m!();
        }
        208 | 209 => {
            pixel00_2!();
            pixel01_1!();
            pixel02_1m!();
            pixel10_1!();
            pixel11!();
            pixel20_1m!();
            if color_diff(w[6], w[8]) {
                pixel12_c!();
                pixel21_c!();
                pixel22_c!();
            } else {
                pixel12_3!();
                pixel21_3!();
                pixel22_4!();
            }
        }
        104 | 108 => {
            pixel00_1m!();
            pixel01_1!();
            pixel02_2!();
            pixel11!();
            pixel12_1!();
            if color_diff(w[8], w[4]) {
                pixel10_c!();
                pixel20_c!();
                pixel21_c!();
            } else {
                pixel10_3!();
                pixel20_4!();
                pixel21_3!();
            }
            pixel22_1m!();
        }
        11 | 139 => {
            if color_diff(w[4], w[2]) {
                pixel00_c!();
                pixel01_c!();
                pixel10_c!();
            } else {
                pixel00_4!();
                pixel01_3!();
                pixel10_3!();
            }
            pixel02_1m!();
            pixel11!();
            pixel12_1!();
            pixel20_1m!();
            pixel21_1!();
            pixel22_2!();
        }
        19 | 51 => {
            if color_diff(w[2], w[6]) {
                pixel00_1l!();
                pixel01_c!();
                pixel02_1m!();
                pixel12_c!();
            } else {
                pixel00_2!();
                pixel01_6!();
                pixel02_5!();
                pixel12_1!();
            }
            pixel10_1!();
            pixel11!();
            pixel20_2!();
            pixel21_1!();
            pixel22_1m!();
        }
        146 | 178 => {
            if color_diff(w[2], w[6]) {
                pixel01_c!();
                pixel02_1m!();
                pixel12_c!();
                pixel22_1d!();
            } else {
                pixel01_1!();
                pixel02_5!();
                pixel12_6!();
                pixel22_2!();
            }
            pixel00_1m!();
            pixel10_1!();
            pixel11!();
            pixel20_2!();
            pixel21_1!();
        }
        84 | 85 => {
            if color_diff(w[6], w[8]) {
                pixel02_1u!();
                pixel12_c!();
                pixel21_c!();
                pixel22_1m!();
            } else {
                pixel02_2!();
                pixel12_6!();
                pixel21_1!();
                pixel22_5!();
            }
            pixel00_2!();
            pixel01_1!();
            pixel10_1!();
            pixel11!();
            pixel20_1m!();
        }
        112 | 113 => {
            if color_diff(w[6], w[8]) {
                pixel12_c!();
                pixel20_1l!();
                pixel21_c!();
                pixel22_1m!();
            } else {
                pixel12_1!();
                pixel20_2!();
                pixel21_6!();
                pixel22_5!();
            }
            pixel00_2!();
            pixel01_1!();
            pixel02_1m!();
            pixel10_1!();
            pixel11!();
        }
        200 | 204 => {
            if color_diff(w[8], w[4]) {
                pixel10_c!();
                pixel20_1m!();
                pixel21_c!();
                pixel22_1r!();
            } else {
                pixel10_1!();
                pixel20_5!();
                pixel21_6!();
                pixel22_2!();
            }
            pixel00_1m!();
            pixel01_1!();
            pixel02_2!();
            pixel11!();
            pixel12_1!();
        }
        73 | 77 => {
            if color_diff(w[8], w[4]) {
                pixel00_1u!();
                pixel10_c!();
                pixel20_1m!();
                pixel21_c!();
            } else {
                pixel00_2!();
                pixel10_6!();
                pixel20_5!();
                pixel21_1!();
            }
            pixel01_1!();
            pixel02_2!();
            pixel11!();
            pixel12_1!();
            pixel22_1m!();
        }
        42 | 170 => {
            if color_diff(w[4], w[2]) {
                pixel00_1m!();
                pixel01_c!();
                pixel10_c!();
                pixel20_1d!();
            } else {
                pixel00_5!();
                pixel01_1!();
                pixel10_6!();
                pixel20_2!();
            }
            pixel02_1m!();
            pixel11!();
            pixel12_1!();
            pixel21_1!();
            pixel22_2!();
        }
        14 | 142 => {
            if color_diff(w[4], w[2]) {
                pixel00_1m!();
                pixel01_c!();
                pixel02_1r!();
                pixel10_c!();
            } else {
                pixel00_5!();
                pixel01_6!();
                pixel02_2!();
                pixel10_1!();
            }
            pixel11!();
            pixel12_1!();
            pixel20_1m!();
            pixel21_1!();
            pixel22_2!();
        }
        67 => {
            pixel00_1l!();
            pixel01_c!();
            pixel02_1m!();
            pixel10_1!();
            pixel11!();
            pixel12_1!();
            pixel20_1m!();
            pixel21_c!();
            pixel22_1m!();
        }
        70 => {
            pixel00_1m!();
            pixel01_c!();
            pixel02_1r!();
            pixel10_1!();
            pixel11!();
            pixel12_1!();
            pixel20_1m!();
            pixel21_c!();
            pixel22_1m!();
        }
        28 => {
            pixel00_1m!();
            pixel01_1!();
            pixel02_1u!();
            pixel10_c!();
            pixel11!();
            pixel12_c!();
            pixel20_1m!();
            pixel21_1!();
            pixel22_1m!();
        }
        152 => {
            pixel00_1m!();
            pixel01_1!();
            pixel02_1m!();
            pixel10_c!();
            pixel11!();
            pixel12_c!();
            pixel20_1m!();
            pixel21_1!();
            pixel22_1d!();
        }
        194 => {
            pixel00_1m!();
            pixel01_c!();
            pixel02_1m!();
            pixel10_1!();
            pixel11!();
            pixel12_1!();
            pixel20_1m!();
            pixel21_c!();
            pixel22_1r!();
        }
        98 => {
            pixel00_1m!();
            pixel01_c!();
            pixel02_1m!();
            pixel10_1!();
            pixel11!();
            pixel12_1!();
            pixel20_1l!();
            pixel21_c!();
            pixel22_1m!();
        }
        56 => {
            pixel00_1m!();
            pixel01_1!();
            pixel02_1m!();
            pixel10_c!();
            pixel11!();
            pixel12_c!();
            pixel20_1d!();
            pixel21_1!();
            pixel22_1m!();
        }
        25 => {
            pixel00_1u!();
            pixel01_1!();
            pixel02_1m!();
            pixel10_c!();
            pixel11!();
            pixel12_c!();
            pixel20_1m!();
            pixel21_1!();
            pixel22_1m!();
        }
        26 | 31 => {
            if color_diff(w[4], w[2]) {
                pixel00_c!();
                pixel10_c!();
            } else {
                pixel00_4!();
                pixel10_3!();
            }
            pixel01_c!();
            if color_diff(w[2], w[6]) {
                pixel02_c!();
                pixel12_c!();
            } else {
                pixel02_4!();
                pixel12_3!();
            }
            pixel11!();
            pixel20_1m!();
            pixel21_1!();
            pixel22_1m!();
        }
        82 | 214 => {
            pixel00_1m!();
            if color_diff(w[2], w[6]) {
                pixel01_c!();
                pixel02_c!();
            } else {
                pixel01_3!();
                pixel02_4!();
            }
            pixel10_1!();
            pixel11!();
            pixel12_c!();
            pixel20_1m!();
            if color_diff(w[6], w[8]) {
                pixel21_c!();
                pixel22_c!();
            } else {
                pixel21_3!();
                pixel22_4!();
            }
        }
        88 | 248 => {
            pixel00_1m!();
            pixel01_1!();
            pixel02_1m!();
            pixel11!();
            if color_diff(w[8], w[4]) {
                pixel10_c!();
                pixel20_c!();
            } else {
                pixel10_3!();
                pixel20_4!();
            }
            pixel21_c!();
            if color_diff(w[6], w[8]) {
                pixel12_c!();
                pixel22_c!();
            } else {
                pixel12_3!();
                pixel22_4!();
            }
        }
        74 | 107 => {
            if color_diff(w[4], w[2]) {
                pixel00_c!();
                pixel01_c!();
            } else {
                pixel00_4!();
                pixel01_3!();
            }
            pixel02_1m!();
            pixel10_c!();
            pixel11!();
            pixel12_1!();
            if color_diff(w[8], w[4]) {
                pixel20_c!();
                pixel21_c!();
            } else {
                pixel20_4!();
                pixel21_3!();
            }
            pixel22_1m!();
        }
        27 => {
            if color_diff(w[4], w[2]) {
                pixel00_c!();
                pixel01_c!();
                pixel10_c!();
            } else {
                pixel00_4!();
                pixel01_3!();
                pixel10_3!();
            }
            pixel02_1m!();
            pixel11!();
            pixel12_c!();
            pixel20_1m!();
            pixel21_1!();
            pixel22_1m!();
        }
        86 => {
            pixel00_1m!();
            if color_diff(w[2], w[6]) {
                pixel01_c!();
                pixel02_c!();
                pixel12_c!();
            } else {
                pixel01_3!();
                pixel02_4!();
                pixel12_3!();
            }
            pixel10_1!();
            pixel11!();
            pixel20_1m!();
            pixel21_c!();
            pixel22_1m!();
        }
        216 => {
            pixel00_1m!();
            pixel01_1!();
            pixel02_1m!();
            pixel10_c!();
            pixel11!();
            pixel20_1m!();
            if color_diff(w[6], w[8]) {
                pixel12_c!();
                pixel21_c!();
                pixel22_c!();
            } else {
                pixel12_3!();
                pixel21_3!();
                pixel22_4!();
            }
        }
        106 => {
            pixel00_1m!();
            pixel01_c!();
            pixel02_1m!();
            pixel11!();
            pixel12_1!();
            if color_diff(w[8], w[4]) {
                pixel10_c!();
                pixel20_c!();
                pixel21_c!();
            } else {
                pixel10_3!();
                pixel20_4!();
                pixel21_3!();
            }
            pixel22_1m!();
        }
        30 => {
            pixel00_1m!();
            if color_diff(w[2], w[6]) {
                pixel01_c!();
                pixel02_c!();
                pixel12_c!();
            } else {
                pixel01_3!();
                pixel02_4!();
                pixel12_3!();
            }
            pixel10_c!();
            pixel11!();
            pixel20_1m!();
            pixel21_1!();
            pixel22_1m!();
        }
        210 => {
            pixel00_1m!();
            pixel01_c!();
            pixel02_1m!();
            pixel10_1!();
            pixel11!();
            pixel20_1m!();
            if color_diff(w[6], w[8]) {
                pixel12_c!();
                pixel21_c!();
                pixel22_c!();
            } else {
                pixel12_3!();
                pixel21_3!();
                pixel22_4!();
            }
        }
        120 => {
            pixel00_1m!();
            pixel01_1!();
            pixel02_1m!();
            pixel11!();
            pixel12_c!();
            if color_diff(w[8], w[4]) {
                pixel10_c!();
                pixel20_c!();
                pixel21_c!();
            } else {
                pixel10_3!();
                pixel20_4!();
                pixel21_3!();
            }
            pixel22_1m!();
        }
        75 => {
            if color_diff(w[4], w[2]) {
                pixel00_c!();
                pixel01_c!();
                pixel10_c!();
            } else {
                pixel00_4!();
                pixel01_3!();
                pixel10_3!();
            }
            pixel02_1m!();
            pixel11!();
            pixel12_1!();
            pixel20_1m!();
            pixel21_c!();
            pixel22_1m!();
        }
        29 => {
            pixel00_1u!();
            pixel01_1!();
            pixel02_1u!();
            pixel10_c!();
            pixel11!();
            pixel12_c!();
            pixel20_1m!();
            pixel21_1!();
            pixel22_1m!();
        }
        198 => {
            pixel00_1m!();
            pixel01_c!();
            pixel02_1r!();
            pixel10_1!();
            pixel11!();
            pixel12_1!();
            pixel20_1m!();
            pixel21_c!();
            pixel22_1r!();
        }
        184 => {
            pixel00_1m!();
            pixel01_1!();
            pixel02_1m!();
            pixel10_c!();
            pixel11!();
            pixel12_c!();
            pixel20_1d!();
            pixel21_1!();
            pixel22_1d!();
        }
        99 => {
            pixel00_1l!();
            pixel01_c!();
            pixel02_1m!();
            pixel10_1!();
            pixel11!();
            pixel12_1!();
            pixel20_1l!();
            pixel21_c!();
            pixel22_1m!();
        }
        57 => {
            pixel00_1u!();
            pixel01_1!();
            pixel02_1m!();
            pixel10_c!();
            pixel11!();
            pixel12_c!();
            pixel20_1d!();
            pixel21_1!();
            pixel22_1m!();
        }
        71 => {
            pixel00_1l!();
            pixel01_c!();
            pixel02_1r!();
            pixel10_1!();
            pixel11!();
            pixel12_1!();
            pixel20_1m!();
            pixel21_c!();
            pixel22_1m!();
        }
        156 => {
            pixel00_1m!();
            pixel01_1!();
            pixel02_1u!();
            pixel10_c!();
            pixel11!();
            pixel12_c!();
            pixel20_1m!();
            pixel21_1!();
            pixel22_1d!();
        }
        226 => {
            pixel00_1m!();
            pixel01_c!();
            pixel02_1m!();
            pixel10_1!();
            pixel11!();
            pixel12_1!();
            pixel20_1l!();
            pixel21_c!();
            pixel22_1r!();
        }
        60 => {
            pixel00_1m!();
            pixel01_1!();
            pixel02_1u!();
            pixel10_c!();
            pixel11!();
            pixel12_c!();
            pixel20_1d!();
            pixel21_1!();
            pixel22_1m!();
        }
        195 => {
            pixel00_1l!();
            pixel01_c!();
            pixel02_1m!();
            pixel10_1!();
            pixel11!();
            pixel12_1!();
            pixel20_1m!();
            pixel21_c!();
            pixel22_1r!();
        }
        102 => {
            pixel00_1m!();
            pixel01_c!();
            pixel02_1r!();
            pixel10_1!();
            pixel11!();
            pixel12_1!();
            pixel20_1l!();
            pixel21_c!();
            pixel22_1m!();
        }
        153 => {
            pixel00_1u!();
            pixel01_1!();
            pixel02_1m!();
            pixel10_c!();
            pixel11!();
            pixel12_c!();
            pixel20_1m!();
            pixel21_1!();
            pixel22_1d!();
        }
        58 => {
            if color_diff(w[4], w[2]) {
                pixel00_1m!();
            } else {
                pixel00_2!();
            }
            pixel01_c!();
            if color_diff(w[2], w[6]) {
                pixel02_1m!();
            } else {
                pixel02_2!();
            }
            pixel10_c!();
            pixel11!();
            pixel12_c!();
            pixel20_1d!();
            pixel21_1!();
            pixel22_1m!();
        }
        83 => {
            pixel00_1l!();
            pixel01_c!();
            if color_diff(w[2], w[6]) {
                pixel02_1m!();
            } else {
                pixel02_2!();
            }
            pixel10_1!();
            pixel11!();
            pixel12_c!();
            pixel20_1m!();
            pixel21_c!();
            if color_diff(w[6], w[8]) {
                pixel22_1m!();
            } else {
                pixel22_2!();
            }
        }
        92 => {
            pixel00_1m!();
            pixel01_1!();
            pixel02_1u!();
            pixel10_c!();
            pixel11!();
            pixel12_c!();
            if color_diff(w[8], w[4]) {
                pixel20_1m!();
            } else {
                pixel20_2!();
            }
            pixel21_c!();
            if color_diff(w[6], w[8]) {
                pixel22_1m!();
            } else {
                pixel22_2!();
            }
        }
        202 => {
            if color_diff(w[4], w[2]) {
                pixel00_1m!();
            } else {
                pixel00_2!();
            }
            pixel01_c!();
            pixel02_1m!();
            pixel10_c!();
            pixel11!();
            pixel12_1!();
            if color_diff(w[8], w[4]) {
                pixel20_1m!();
            } else {
                pixel20_2!();
            }
            pixel21_c!();
            pixel22_1r!();
        }
        78 => {
            if color_diff(w[4], w[2]) {
                pixel00_1m!();
            } else {
                pixel00_2!();
            }
            pixel01_c!();
            pixel02_1r!();
            pixel10_c!();
            pixel11!();
            pixel12_1!();
            if color_diff(w[8], w[4]) {
                pixel20_1m!();
            } else {
                pixel20_2!();
            }
            pixel21_c!();
            pixel22_1m!();
        }
        154 => {
            if color_diff(w[4], w[2]) {
                pixel00_1m!();
            } else {
                pixel00_2!();
            }
            pixel01_c!();
            if color_diff(w[2], w[6]) {
                pixel02_1m!();
            } else {
                pixel02_2!();
            }
            pixel10_c!();
            pixel11!();
            pixel12_c!();
            pixel20_1m!();
            pixel21_1!();
            pixel22_1d!();
        }
        114 => {
            pixel00_1m!();
            pixel01_c!();
            if color_diff(w[2], w[6]) {
                pixel02_1m!();
            } else {
                pixel02_2!();
            }
            pixel10_1!();
            pixel11!();
            pixel12_c!();
            pixel20_1l!();
            pixel21_c!();
            if color_diff(w[6], w[8]) {
                pixel22_1m!();
            } else {
                pixel22_2!();
            }
        }
        89 => {
            pixel00_1u!();
            pixel01_1!();
            pixel02_1m!();
            pixel10_c!();
            pixel11!();
            pixel12_c!();
            if color_diff(w[8], w[4]) {
                pixel20_1m!();
            } else {
                pixel20_2!();
            }
            pixel21_c!();
            if color_diff(w[6], w[8]) {
                pixel22_1m!();
            } else {
                pixel22_2!();
            }
        }
        90 => {
            if color_diff(w[4], w[2]) {
                pixel00_1m!();
            } else {
                pixel00_2!();
            }
            pixel01_c!();
            if color_diff(w[2], w[6]) {
                pixel02_1m!();
            } else {
                pixel02_2!();
            }
            pixel10_c!();
            pixel11!();
            pixel12_c!();
            if color_diff(w[8], w[4]) {
                pixel20_1m!();
            } else {
                pixel20_2!();
            }
            pixel21_c!();
            if color_diff(w[6], w[8]) {
                pixel22_1m!();
            } else {
                pixel22_2!();
            }
        }
        55 | 23 => {
            if color_diff(w[2], w[6]) {
                pixel00_1l!();
                pixel01_c!();
                pixel02_c!();
                pixel12_c!();
            } else {
                pixel00_2!();
                pixel01_6!();
                pixel02_5!();
                pixel12_1!();
            }
            pixel10_1!();
            pixel11!();
            pixel20_2!();
            pixel21_1!();
            pixel22_1m!();
        }
        182 | 150 => {
            if color_diff(w[2], w[6]) {
                pixel01_c!();
                pixel02_c!();
                pixel12_c!();
                pixel22_1d!();
            } else {
                pixel01_1!();
                pixel02_5!();
                pixel12_6!();
                pixel22_2!();
            }
            pixel00_1m!();
            pixel10_1!();
            pixel11!();
            pixel20_2!();
            pixel21_1!();
        }
        213 | 212 => {
            if color_diff(w[6], w[8]) {
                pixel02_1u!();
                pixel12_c!();
                pixel21_c!();
                pixel22_c!();
            } else {
                pixel02_2!();
                pixel12_6!();
                pixel21_1!();
                pixel22_5!();
            }
            pixel00_2!();
            pixel01_1!();
            pixel10_1!();
            pixel11!();
            pixel20_1m!();
        }
        241 | 240 => {
            if color_diff(w[6], w[8]) {
                pixel12_c!();
                pixel20_1l!();
                pixel21_c!();
                pixel22_c!();
            } else {
                pixel12_1!();
                pixel20_2!();
                pixel21_6!();
                pixel22_5!();
            }
            pixel00_2!();
            pixel01_1!();
            pixel02_1m!();
            pixel10_1!();
            pixel11!();
        }
        236 | 232 => {
            if color_diff(w[8], w[4]) {
                pixel10_c!();
                pixel20_c!();
                pixel21_c!();
                pixel22_1r!();
            } else {
                pixel10_1!();
                pixel20_5!();
                pixel21_6!();
                pixel22_2!();
            }
            pixel00_1m!();
            pixel01_1!();
            pixel02_2!();
            pixel11!();
            pixel12_1!();
        }
        109 | 105 => {
            if color_diff(w[8], w[4]) {
                pixel00_1u!();
                pixel10_c!();
                pixel20_c!();
                pixel21_c!();
            } else {
                pixel00_2!();
                pixel10_6!();
                pixel20_5!();
                pixel21_1!();
            }
            pixel01_1!();
            pixel02_2!();
            pixel11!();
            pixel12_1!();
            pixel22_1m!();
        }
        171 | 43 => {
            if color_diff(w[4], w[2]) {
                pixel00_c!();
                pixel01_c!();
                pixel10_c!();
                pixel20_1d!();
            } else {
                pixel00_5!();
                pixel01_1!();
                pixel10_6!();
                pixel20_2!();
            }
            pixel02_1m!();
            pixel11!();
            pixel12_1!();
            pixel21_1!();
            pixel22_2!();
        }
        143 | 15 => {
            if color_diff(w[4], w[2]) {
                pixel00_c!();
                pixel01_c!();
                pixel02_1r!();
                pixel10_c!();
            } else {
                pixel00_5!();
                pixel01_6!();
                pixel02_2!();
                pixel10_1!();
            }
            pixel11!();
            pixel12_1!();
            pixel20_1m!();
            pixel21_1!();
            pixel22_2!();
        }
        124 => {
            pixel00_1m!();
            pixel01_1!();
            pixel02_1u!();
            pixel11!();
            pixel12_c!();
            if color_diff(w[8], w[4]) {
                pixel10_c!();
                pixel20_c!();
                pixel21_c!();
            } else {
                pixel10_3!();
                pixel20_4!();
                pixel21_3!();
            }
            pixel22_1m!();
        }
        203 => {
            if color_diff(w[4], w[2]) {
                pixel00_c!();
                pixel01_c!();
                pixel10_c!();
            } else {
                pixel00_4!();
                pixel01_3!();
                pixel10_3!();
            }
            pixel02_1m!();
            pixel11!();
            pixel12_1!();
            pixel20_1m!();
            pixel21_c!();
            pixel22_1r!();
        }
        62 => {
            pixel00_1m!();
            if color_diff(w[2], w[6]) {
                pixel01_c!();
                pixel02_c!();
                pixel12_c!();
            } else {
                pixel01_3!();
                pixel02_4!();
                pixel12_3!();
            }
            pixel10_c!();
            pixel11!();
            pixel20_1d!();
            pixel21_1!();
            pixel22_1m!();
        }
        211 => {
            pixel00_1l!();
            pixel01_c!();
            pixel02_1m!();
            pixel10_1!();
            pixel11!();
            pixel20_1m!();
            if color_diff(w[6], w[8]) {
                pixel12_c!();
                pixel21_c!();
                pixel22_c!();
            } else {
                pixel12_3!();
                pixel21_3!();
                pixel22_4!();
            }
        }
        118 => {
            pixel00_1m!();
            if color_diff(w[2], w[6]) {
                pixel01_c!();
                pixel02_c!();
                pixel12_c!();
            } else {
                pixel01_3!();
                pixel02_4!();
                pixel12_3!();
            }
            pixel10_1!();
            pixel11!();
            pixel20_1l!();
            pixel21_c!();
            pixel22_1m!();
        }
        217 => {
            pixel00_1u!();
            pixel01_1!();
            pixel02_1m!();
            pixel10_c!();
            pixel11!();
            pixel20_1m!();
            if color_diff(w[6], w[8]) {
                pixel12_c!();
                pixel21_c!();
                pixel22_c!();
            } else {
                pixel12_3!();
                pixel21_3!();
                pixel22_4!();
            }
        }
        110 => {
            pixel00_1m!();
            pixel01_c!();
            pixel02_1r!();
            pixel11!();
            pixel12_1!();
            if color_diff(w[8], w[4]) {
                pixel10_c!();
                pixel20_c!();
                pixel21_c!();
            } else {
                pixel10_3!();
                pixel20_4!();
                pixel21_3!();
            }
            pixel22_1m!();
        }
        155 => {
            if color_diff(w[4], w[2]) {
                pixel00_c!();
                pixel01_c!();
                pixel10_c!();
            } else {
                pixel00_4!();
                pixel01_3!();
                pixel10_3!();
            }
            pixel02_1m!();
            pixel11!();
            pixel12_c!();
            pixel20_1m!();
            pixel21_1!();
            pixel22_1d!();
        }
        188 => {
            pixel00_1m!();
            pixel01_1!();
            pixel02_1u!();
            pixel10_c!();
            pixel11!();
            pixel12_c!();
            pixel20_1d!();
            pixel21_1!();
            pixel22_1d!();
        }
        185 => {
            pixel00_1u!();
            pixel01_1!();
            pixel02_1m!();
            pixel10_c!();
            pixel11!();
            pixel12_c!();
            pixel20_1d!();
            pixel21_1!();
            pixel22_1d!();
        }
        61 => {
            pixel00_1u!();
            pixel01_1!();
            pixel02_1u!();
            pixel10_c!();
            pixel11!();
            pixel12_c!();
            pixel20_1d!();
            pixel21_1!();
            pixel22_1m!();
        }
        157 => {
            pixel00_1u!();
            pixel01_1!();
            pixel02_1u!();
            pixel10_c!();
            pixel11!();
            pixel12_c!();
            pixel20_1m!();
            pixel21_1!();
            pixel22_1d!();
        }
        103 => {
            pixel00_1l!();
            pixel01_c!();
            pixel02_1r!();
            pixel10_1!();
            pixel11!();
            pixel12_1!();
            pixel20_1l!();
            pixel21_c!();
            pixel22_1m!();
        }
        227 => {
            pixel00_1l!();
            pixel01_c!();
            pixel02_1m!();
            pixel10_1!();
            pixel11!();
            pixel12_1!();
            pixel20_1l!();
            pixel21_c!();
            pixel22_1r!();
        }
        230 => {
            pixel00_1m!();
            pixel01_c!();
            pixel02_1r!();
            pixel10_1!();
            pixel11!();
            pixel12_1!();
            pixel20_1l!();
            pixel21_c!();
            pixel22_1r!();
        }
        199 => {
            pixel00_1l!();
            pixel01_c!();
            pixel02_1r!();
            pixel10_1!();
            pixel11!();
            pixel12_1!();
            pixel20_1m!();
            pixel21_c!();
            pixel22_1r!();
        }
        220 => {
            pixel00_1m!();
            pixel01_1!();
            pixel02_1u!();
            pixel10_c!();
            pixel11!();
            if color_diff(w[8], w[4]) {
                pixel20_1m!();
            } else {
                pixel20_2!();
            }
            if color_diff(w[6], w[8]) {
                pixel12_c!();
                pixel21_c!();
                pixel22_c!();
            } else {
                pixel12_3!();
                pixel21_3!();
                pixel22_4!();
            }
        }
        158 => {
            if color_diff(w[4], w[2]) {
                pixel00_1m!();
            } else {
                pixel00_2!();
            }
            if color_diff(w[2], w[6]) {
                pixel01_c!();
                pixel02_c!();
                pixel12_c!();
            } else {
                pixel01_3!();
                pixel02_4!();
                pixel12_3!();
            }
            pixel10_c!();
            pixel11!();
            pixel20_1m!();
            pixel21_1!();
            pixel22_1d!();
        }
        234 => {
            if color_diff(w[4], w[2]) {
                pixel00_1m!();
            } else {
                pixel00_2!();
            }
            pixel01_c!();
            pixel02_1m!();
            pixel11!();
            pixel12_1!();
            if color_diff(w[8], w[4]) {
                pixel10_c!();
                pixel20_c!();
                pixel21_c!();
            } else {
                pixel10_3!();
                pixel20_4!();
                pixel21_3!();
            }
            pixel22_1r!();
        }
        242 => {
            pixel00_1m!();
            pixel01_c!();
            if color_diff(w[2], w[6]) {
                pixel02_1m!();
            } else {
                pixel02_2!();
            }
            pixel10_1!();
            pixel11!();
            pixel20_1l!();
            if color_diff(w[6], w[8]) {
                pixel12_c!();
                pixel21_c!();
                pixel22_c!();
            } else {
                pixel12_3!();
                pixel21_3!();
                pixel22_4!();
            }
        }
        59 => {
            if color_diff(w[4], w[2]) {
                pixel00_c!();
                pixel01_c!();
                pixel10_c!();
            } else {
                pixel00_4!();
                pixel01_3!();
                pixel10_3!();
            }
            if color_diff(w[2], w[6]) {
                pixel02_1m!();
            } else {
                pixel02_2!();
            }
            pixel11!();
            pixel12_c!();
            pixel20_1d!();
            pixel21_1!();
            pixel22_1m!();
        }
        121 => {
            pixel00_1u!();
            pixel01_1!();
            pixel02_1m!();
            pixel11!();
            pixel12_c!();
            if color_diff(w[8], w[4]) {
                pixel10_c!();
                pixel20_c!();
                pixel21_c!();
            } else {
                pixel10_3!();
                pixel20_4!();
                pixel21_3!();
            }
            if color_diff(w[6], w[8]) {
                pixel22_1m!();
            } else {
                pixel22_2!();
            }
        }
        87 => {
            pixel00_1l!();
            if color_diff(w[2], w[6]) {
                pixel01_c!();
                pixel02_c!();
                pixel12_c!();
            } else {
                pixel01_3!();
                pixel02_4!();
                pixel12_3!();
            }
            pixel10_1!();
            pixel11!();
            pixel20_1m!();
            pixel21_c!();
            if color_diff(w[6], w[8]) {
                pixel22_1m!();
            } else {
                pixel22_2!();
            }
        }
        79 => {
            if color_diff(w[4], w[2]) {
                pixel00_c!();
                pixel01_c!();
                pixel10_c!();
            } else {
                pixel00_4!();
                pixel01_3!();
                pixel10_3!();
            }
            pixel02_1r!();
            pixel11!();
            pixel12_1!();
            if color_diff(w[8], w[4]) {
                pixel20_1m!();
            } else {
                pixel20_2!();
            }
            pixel21_c!();
            pixel22_1m!();
        }
        122 => {
            if color_diff(w[4], w[2]) {
                pixel00_1m!();
            } else {
                pixel00_2!();
            }
            pixel01_c!();
            if color_diff(w[2], w[6]) {
                pixel02_1m!();
            } else {
                pixel02_2!();
            }
            pixel11!();
            pixel12_c!();
            if color_diff(w[8], w[4]) {
                pixel10_c!();
                pixel20_c!();
                pixel21_c!();
            } else {
                pixel10_3!();
                pixel20_4!();
                pixel21_3!();
            }
            if color_diff(w[6], w[8]) {
                pixel22_1m!();
            } else {
                pixel22_2!();
            }
        }
        94 => {
            if color_diff(w[4], w[2]) {
                pixel00_1m!();
            } else {
                pixel00_2!();
            }
            if color_diff(w[2], w[6]) {
                pixel01_c!();
                pixel02_c!();
                pixel12_c!();
            } else {
                pixel01_3!();
                pixel02_4!();
                pixel12_3!();
            }
            pixel10_c!();
            pixel11!();
            if color_diff(w[8], w[4]) {
                pixel20_1m!();
            } else {
                pixel20_2!();
            }
            pixel21_c!();
            if color_diff(w[6], w[8]) {
                pixel22_1m!();
            } else {
                pixel22_2!();
            }
        }
        218 => {
            if color_diff(w[4], w[2]) {
                pixel00_1m!();
            } else {
                pixel00_2!();
            }
            pixel01_c!();
            if color_diff(w[2], w[6]) {
                pixel02_1m!();
            } else {
                pixel02_2!();
            }
            pixel10_c!();
            pixel11!();
            if color_diff(w[8], w[4]) {
                pixel20_1m!();
            } else {
                pixel20_2!();
            }
            if color_diff(w[6], w[8]) {
                pixel12_c!();
                pixel21_c!();
                pixel22_c!();
            } else {
                pixel12_3!();
                pixel21_3!();
                pixel22_4!();
            }
        }
        91 => {
            if color_diff(w[4], w[2]) {
                pixel00_c!();
                pixel01_c!();
                pixel10_c!();
            } else {
                pixel00_4!();
                pixel01_3!();
                pixel10_3!();
            }
            if color_diff(w[2], w[6]) {
                pixel02_1m!();
            } else {
                pixel02_2!();
            }
            pixel11!();
            pixel12_c!();
            if color_diff(w[8], w[4]) {
                pixel20_1m!();
            } else {
                pixel20_2!();
            }
            pixel21_c!();
            if color_diff(w[6], w[8]) {
                pixel22_1m!();
            } else {
                pixel22_2!();
            }
        }
        229 => {
            pixel00_2!();
            pixel01_1!();
            pixel02_2!();
            pixel10_1!();
            pixel11!();
            pixel12_1!();
            pixel20_1l!();
            pixel21_c!();
            pixel22_1r!();
        }
        167 => {
            pixel00_1l!();
            pixel01_c!();
            pixel02_1r!();
            pixel10_1!();
            pixel11!();
            pixel12_1!();
            pixel20_2!();
            pixel21_1!();
            pixel22_2!();
        }
        173 => {
            pixel00_1u!();
            pixel01_1!();
            pixel02_2!();
            pixel10_c!();
            pixel11!();
            pixel12_1!();
            pixel20_1d!();
            pixel21_1!();
            pixel22_2!();
        }
        181 => {
            pixel00_2!();
            pixel01_1!();
            pixel02_1u!();
            pixel10_1!();
            pixel11!();
            pixel12_c!();
            pixel20_2!();
            pixel21_1!();
            pixel22_1d!();
        }
        186 => {
            if color_diff(w[4], w[2]) {
                pixel00_1m!();
            } else {
                pixel00_2!();
            }
            pixel01_c!();
            if color_diff(w[2], w[6]) {
                pixel02_1m!();
            } else {
                pixel02_2!();
            }
            pixel10_c!();
            pixel11!();
            pixel12_c!();
            pixel20_1d!();
            pixel21_1!();
            pixel22_1d!();
        }
        115 => {
            pixel00_1l!();
            pixel01_c!();
            if color_diff(w[2], w[6]) {
                pixel02_1m!();
            } else {
                pixel02_2!();
            }
            pixel10_1!();
            pixel11!();
            pixel12_c!();
            pixel20_1l!();
            pixel21_c!();
            if color_diff(w[6], w[8]) {
                pixel22_1m!();
            } else {
                pixel22_2!();
            }
        }
        93 => {
            pixel00_1u!();
            pixel01_1!();
            pixel02_1u!();
            pixel10_c!();
            pixel11!();
            pixel12_c!();
            if color_diff(w[8], w[4]) {
                pixel20_1m!();
            } else {
                pixel20_2!();
            }
            pixel21_c!();
            if color_diff(w[6], w[8]) {
                pixel22_1m!();
            } else {
                pixel22_2!();
            }
        }
        206 => {
            if color_diff(w[4], w[2]) {
                pixel00_1m!();
            } else {
                pixel00_2!();
            }
            pixel01_c!();
            pixel02_1r!();
            pixel10_c!();
            pixel11!();
            pixel12_1!();
            if color_diff(w[8], w[4]) {
                pixel20_1m!();
            } else {
                pixel20_2!();
            }
            pixel21_c!();
            pixel22_1r!();
        }
        205 | 201 => {
            pixel00_1u!();
            pixel01_1!();
            pixel02_2!();
            pixel10_c!();
            pixel11!();
            pixel12_1!();
            if color_diff(w[8], w[4]) {
                pixel20_1m!();
            } else {
                pixel20_2!();
            }
            pixel21_c!();
            pixel22_1r!();
        }
        174 | 46 => {
            if color_diff(w[4], w[2]) {
                pixel00_1m!();
            } else {
                pixel00_2!();
            }
            pixel01_c!();
            pixel02_1r!();
            pixel10_c!();
            pixel11!();
            pixel12_1!();
            pixel20_1d!();
            pixel21_1!();
            pixel22_2!();
        }
        179 | 147 => {
            pixel00_1l!();
            pixel01_c!();
            if color_diff(w[2], w[6]) {
                pixel02_1m!();
            } else {
                pixel02_2!();
            }
            pixel10_1!();
            pixel11!();
            pixel12_c!();
            pixel20_2!();
            pixel21_1!();
            pixel22_1d!();
        }
        117 | 116 => {
            pixel00_2!();
            pixel01_1!();
            pixel02_1u!();
            pixel10_1!();
            pixel11!();
            pixel12_c!();
            pixel20_1l!();
            pixel21_c!();
            if color_diff(w[6], w[8]) {
                pixel22_1m!();
            } else {
                pixel22_2!();
            }
        }
        189 => {
            pixel00_1u!();
            pixel01_1!();
            pixel02_1u!();
            pixel10_c!();
            pixel11!();
            pixel12_c!();
            pixel20_1d!();
            pixel21_1!();
            pixel22_1d!();
        }
        231 => {
            pixel00_1l!();
            pixel01_c!();
            pixel02_1r!();
            pixel10_1!();
            pixel11!();
            pixel12_1!();
            pixel20_1l!();
            pixel21_c!();
            pixel22_1r!();
        }
        126 => {
            pixel00_1m!();
            if color_diff(w[2], w[6]) {
                pixel01_c!();
                pixel02_c!();
                pixel12_c!();
            } else {
                pixel01_3!();
                pixel02_4!();
                pixel12_3!();
            }
            pixel11!();
            if color_diff(w[8], w[4]) {
                pixel10_c!();
                pixel20_c!();
                pixel21_c!();
            } else {
                pixel10_3!();
                pixel20_4!();
                pixel21_3!();
            }
            pixel22_1m!();
        }
        219 => {
            if color_diff(w[4], w[2]) {
                pixel00_c!();
                pixel01_c!();
                pixel10_c!();
            } else {
                pixel00_4!();
                pixel01_3!();
                pixel10_3!();
            }
            pixel02_1m!();
            pixel11!();
            pixel20_1m!();
            if color_diff(w[6], w[8]) {
                pixel12_c!();
                pixel21_c!();
                pixel22_c!();
            } else {
                pixel12_3!();
                pixel21_3!();
                pixel22_4!();
            }
        }
        125 => {
            if color_diff(w[8], w[4]) {
                pixel00_1u!();
                pixel10_c!();
                pixel20_c!();
                pixel21_c!();
            } else {
                pixel00_2!();
                pixel10_6!();
                pixel20_5!();
                pixel21_1!();
            }
            pixel01_1!();
            pixel02_1u!();
            pixel11!();
            pixel12_c!();
            pixel22_1m!();
        }
        221 => {
            if color_diff(w[6], w[8]) {
                pixel02_1u!();
                pixel12_c!();
                pixel21_c!();
                pixel22_c!();
            } else {
                pixel02_2!();
                pixel12_6!();
                pixel21_1!();
                pixel22_5!();
            }
            pixel00_1u!();
            pixel01_1!();
            pixel10_c!();
            pixel11!();
            pixel20_1m!();
        }
        207 => {
            if color_diff(w[4], w[2]) {
                pixel00_c!();
                pixel01_c!();
                pixel02_1r!();
                pixel10_c!();
            } else {
                pixel00_5!();
                pixel01_6!();
                pixel02_2!();
                pixel10_1!();
            }
            pixel11!();
            pixel12_1!();
            pixel20_1m!();
            pixel21_c!();
            pixel22_1r!();
        }
        238 => {
            if color_diff(w[8], w[4]) {
                pixel10_c!();
                pixel20_c!();
                pixel21_c!();
                pixel22_1r!();
            } else {
                pixel10_1!();
                pixel20_5!();
                pixel21_6!();
                pixel22_2!();
            }
            pixel00_1m!();
            pixel01_c!();
            pixel02_1r!();
            pixel11!();
            pixel12_1!();
        }
        190 => {
            if color_diff(w[2], w[6]) {
                pixel01_c!();
                pixel02_c!();
                pixel12_c!();
                pixel22_1d!();
            } else {
                pixel01_1!();
                pixel02_5!();
                pixel12_6!();
                pixel22_2!();
            }
            pixel00_1m!();
            pixel10_c!();
            pixel11!();
            pixel20_1d!();
            pixel21_1!();
        }
        187 => {
            if color_diff(w[4], w[2]) {
                pixel00_c!();
                pixel01_c!();
                pixel10_c!();
                pixel20_1d!();
            } else {
                pixel00_5!();
                pixel01_1!();
                pixel10_6!();
                pixel20_2!();
            }
            pixel02_1m!();
            pixel11!();
            pixel12_c!();
            pixel21_1!();
            pixel22_1d!();
        }
        243 => {
            if color_diff(w[6], w[8]) {
                pixel12_c!();
                pixel20_1l!();
                pixel21_c!();
                pixel22_c!();
            } else {
                pixel12_1!();
                pixel20_2!();
                pixel21_6!();
                pixel22_5!();
            }
            pixel00_1l!();
            pixel01_c!();
            pixel02_1m!();
            pixel10_1!();
            pixel11!();
        }
        119 => {
            if color_diff(w[2], w[6]) {
                pixel00_1l!();
                pixel01_c!();
                pixel02_c!();
                pixel12_c!();
            } else {
                pixel00_2!();
                pixel01_6!();
                pixel02_5!();
                pixel12_1!();
            }
            pixel10_1!();
            pixel11!();
            pixel20_1l!();
            pixel21_c!();
            pixel22_1m!();
        }
        237 | 233 => {
            pixel00_1u!();
            pixel01_1!();
            pixel02_2!();
            pixel10_c!();
            pixel11!();
            pixel12_1!();
            if color_diff(w[8], w[4]) {
                pixel20_c!();
            } else {
                pixel20_2!();
            }
            pixel21_c!();
            pixel22_1r!();
        }
        175 | 47 => {
            if color_diff(w[4], w[2]) {
                pixel00_c!();
            } else {
                pixel00_2!();
            }
            pixel01_c!();
            pixel02_1r!();
            pixel10_c!();
            pixel11!();
            pixel12_1!();
            pixel20_1d!();
            pixel21_1!();
            pixel22_2!();
        }
        183 | 151 => {
            pixel00_1l!();
            pixel01_c!();
            if color_diff(w[2], w[6]) {
                pixel02_c!();
            } else {
                pixel02_2!();
            }
            pixel10_1!();
            pixel11!();
            pixel12_c!();
            pixel20_2!();
            pixel21_1!();
            pixel22_1d!();
        }
        245 | 244 => {
            pixel00_2!();
            pixel01_1!();
            pixel02_1u!();
            pixel10_1!();
            pixel11!();
            pixel12_c!();
            pixel20_1l!();
            pixel21_c!();
            if color_diff(w[6], w[8]) {
                pixel22_c!();
            } else {
                pixel22_2!();
            }
        }
        250 => {
            pixel00_1m!();
            pixel01_c!();
            pixel02_1m!();
            pixel11!();
            if color_diff(w[8], w[4]) {
                pixel10_c!();
                pixel20_c!();
            } else {
                pixel10_3!();
                pixel20_4!();
            }
            pixel21_c!();
            if color_diff(w[6], w[8]) {
                pixel12_c!();
                pixel22_c!();
            } else {
                pixel12_3!();
                pixel22_4!();
            }
        }
        123 => {
            if color_diff(w[4], w[2]) {
                pixel00_c!();
                pixel01_c!();
            } else {
                pixel00_4!();
                pixel01_3!();
            }
            pixel02_1m!();
            pixel10_c!();
            pixel11!();
            pixel12_c!();
            if color_diff(w[8], w[4]) {
                pixel20_c!();
                pixel21_c!();
            } else {
                pixel20_4!();
                pixel21_3!();
            }
            pixel22_1m!();
        }
        95 => {
            if color_diff(w[4], w[2]) {
                pixel00_c!();
                pixel10_c!();
            } else {
                pixel00_4!();
                pixel10_3!();
            }
            pixel01_c!();
            if color_diff(w[2], w[6]) {
                pixel02_c!();
                pixel12_c!();
            } else {
                pixel02_4!();
                pixel12_3!();
            }
            pixel11!();
            pixel20_1m!();
            pixel21_c!();
            pixel22_1m!();
        }
        222 => {
            pixel00_1m!();
            if color_diff(w[2], w[6]) {
                pixel01_c!();
                pixel02_c!();
            } else {
                pixel01_3!();
                pixel02_4!();
            }
            pixel10_c!();
            pixel11!();
            pixel12_c!();
            pixel20_1m!();
            if color_diff(w[6], w[8]) {
                pixel21_c!();
                pixel22_c!();
            } else {
                pixel21_3!();
                pixel22_4!();
            }
        }
        252 => {
            pixel00_1m!();
            pixel01_1!();
            pixel02_1u!();
            pixel11!();
            pixel12_c!();
            if color_diff(w[8], w[4]) {
                pixel10_c!();
                pixel20_c!();
            } else {
                pixel10_3!();
                pixel20_4!();
            }
            pixel21_c!();
            if color_diff(w[6], w[8]) {
                pixel22_c!();
            } else {
                pixel22_2!();
            }
        }
        249 => {
            pixel00_1u!();
            pixel01_1!();
            pixel02_1m!();
            pixel10_c!();
            pixel11!();
            if color_diff(w[8], w[4]) {
                pixel20_c!();
            } else {
                pixel20_2!();
            }
            pixel21_c!();
            if color_diff(w[6], w[8]) {
                pixel12_c!();
                pixel22_c!();
            } else {
                pixel12_3!();
                pixel22_4!();
            }
        }
        235 => {
            if color_diff(w[4], w[2]) {
                pixel00_c!();
                pixel01_c!();
            } else {
                pixel00_4!();
                pixel01_3!();
            }
            pixel02_1m!();
            pixel10_c!();
            pixel11!();
            pixel12_1!();
            if color_diff(w[8], w[4]) {
                pixel20_c!();
            } else {
                pixel20_2!();
            }
            pixel21_c!();
            pixel22_1r!();
        }
        111 => {
            if color_diff(w[4], w[2]) {
                pixel00_c!();
            } else {
                pixel00_2!();
            }
            pixel01_c!();
            pixel02_1r!();
            pixel10_c!();
            pixel11!();
            pixel12_1!();
            if color_diff(w[8], w[4]) {
                pixel20_c!();
                pixel21_c!();
            } else {
                pixel20_4!();
                pixel21_3!();
            }
            pixel22_1m!();
        }
        63 => {
            if color_diff(w[4], w[2]) {
                pixel00_c!();
            } else {
                pixel00_2!();
            }
            pixel01_c!();
            if color_diff(w[2], w[6]) {
                pixel02_c!();
                pixel12_c!();
            } else {
                pixel02_4!();
                pixel12_3!();
            }
            pixel10_c!();
            pixel11!();
            pixel20_1d!();
            pixel21_1!();
            pixel22_1m!();
        }
        159 => {
            if color_diff(w[4], w[2]) {
                pixel00_c!();
                pixel10_c!();
            } else {
                pixel00_4!();
                pixel10_3!();
            }
            pixel01_c!();
            if color_diff(w[2], w[6]) {
                pixel02_c!();
            } else {
                pixel02_2!();
            }
            pixel11!();
            pixel12_c!();
            pixel20_1m!();
            pixel21_1!();
            pixel22_1d!();
        }
        215 => {
            pixel00_1l!();
            pixel01_c!();
            if color_diff(w[2], w[6]) {
                pixel02_c!();
            } else {
                pixel02_2!();
            }
            pixel10_1!();
            pixel11!();
            pixel12_c!();
            pixel20_1m!();
            if color_diff(w[6], w[8]) {
                pixel21_c!();
                pixel22_c!();
            } else {
                pixel21_3!();
                pixel22_4!();
            }
        }
        246 => {
            pixel00_1m!();
            if color_diff(w[2], w[6]) {
                pixel01_c!();
                pixel02_c!();
            } else {
                pixel01_3!();
                pixel02_4!();
            }
            pixel10_1!();
            pixel11!();
            pixel12_c!();
            pixel20_1l!();
            pixel21_c!();
            if color_diff(w[6], w[8]) {
                pixel22_c!();
            } else {
                pixel22_2!();
            }
        }
        254 => {
            pixel00_1m!();
            if color_diff(w[2], w[6]) {
                pixel01_c!();
                pixel02_c!();
            } else {
                pixel01_3!();
                pixel02_4!();
            }
            pixel11!();
            if color_diff(w[8], w[4]) {
                pixel10_c!();
                pixel20_c!();
            } else {
                pixel10_3!();
                pixel20_4!();
            }
            if color_diff(w[6], w[8]) {
                pixel12_c!();
                pixel21_c!();
                pixel22_c!();
            } else {
                pixel12_3!();
                pixel21_3!();
                pixel22_2!();
            }
        }
        253 => {
            pixel00_1u!();
            pixel01_1!();
            pixel02_1u!();
            pixel10_c!();
            pixel11!();
            pixel12_c!();
            if color_diff(w[8], w[4]) {
                pixel20_c!();
            } else {
                pixel20_2!();
            }
            pixel21_c!();
            if color_diff(w[6], w[8]) {
                pixel22_c!();
            } else {
                pixel22_2!();
            }
        }
        251 => {
            if color_diff(w[4], w[2]) {
                pixel00_c!();
                pixel01_c!();
            } else {
                pixel00_4!();
                pixel01_3!();
            }
            pixel02_1m!();
            pixel11!();
            if color_diff(w[8], w[4]) {
                pixel10_c!();
                pixel20_c!();
                pixel21_c!();
            } else {
                pixel10_3!();
                pixel20_2!();
                pixel21_3!();
            }
            if color_diff(w[6], w[8]) {
                pixel12_c!();
                pixel22_c!();
            } else {
                pixel12_3!();
                pixel22_4!();
            }
        }
        239 => {
            if color_diff(w[4], w[2]) {
                pixel00_c!();
            } else {
                pixel00_2!();
            }
            pixel01_c!();
            pixel02_1r!();
            pixel10_c!();
            pixel11!();
            pixel12_1!();
            if color_diff(w[8], w[4]) {
                pixel20_c!();
            } else {
                pixel20_2!();
            }
            pixel21_c!();
            pixel22_1r!();
        }
        127 => {
            if color_diff(w[4], w[2]) {
                pixel00_c!();
                pixel01_c!();
                pixel10_c!();
            } else {
                pixel00_2!();
                pixel01_3!();
                pixel10_3!();
            }
            if color_diff(w[2], w[6]) {
                pixel02_c!();
                pixel12_c!();
            } else {
                pixel02_4!();
                pixel12_3!();
            }
            pixel11!();
            if color_diff(w[8], w[4]) {
                pixel20_c!();
                pixel21_c!();
            } else {
                pixel20_4!();
                pixel21_3!();
            }
            pixel22_1m!();
        }
        191 => {
            if color_diff(w[4], w[2]) {
                pixel00_c!();
            } else {
                pixel00_2!();
            }
            pixel01_c!();
            if color_diff(w[2], w[6]) {
                pixel02_c!();
            } else {
                pixel02_2!();
            }
            pixel10_c!();
            pixel11!();
            pixel12_c!();
            pixel20_1d!();
            pixel21_1!();
            pixel22_1d!();
        }
        223 => {
            if color_diff(w[4], w[2]) {
                pixel00_c!();
                pixel10_c!();
            } else {
                pixel00_4!();
                pixel10_3!();
            }
            if color_diff(w[2], w[6]) {
                pixel01_c!();
                pixel02_c!();
                pixel12_c!();
            } else {
                pixel01_3!();
                pixel02_2!();
                pixel12_3!();
            }
            pixel11!();
            pixel20_1m!();
            if color_diff(w[6], w[8]) {
                pixel21_c!();
                pixel22_c!();
            } else {
                pixel21_3!();
                pixel22_4!();
            }
        }
        247 => {
            pixel00_1l!();
            pixel01_c!();
            if color_diff(w[2], w[6]) {
                pixel02_c!();
            } else {
                pixel02_2!();
            }
            pixel10_1!();
            pixel11!();
            pixel12_c!();
            pixel20_1l!();
            pixel21_c!();
            if color_diff(w[6], w[8]) {
                pixel22_c!();
            } else {
                pixel22_2!();
            }
        }
        255 => {
            if color_diff(w[4], w[2]) {
                pixel00_c!();
            } else {
                pixel00_2!();
            }
            pixel01_c!();
            if color_diff(w[2], w[6]) {
                pixel02_c!();
            } else {
                pixel02_2!();
            }
            pixel10_c!();
            pixel11!();
            pixel12_c!();
            if color_diff(w[8], w[4]) {
                pixel20_c!();
            } else {
                pixel20_2!();
            }
            pixel21_c!();
            if color_diff(w[6], w[8]) {
                pixel22_c!();
            } else {
                pixel22_2!();
            }
        }
        _ => unreachable!(),
    }
}

fn hq4x_inner(
    w: &[Color; 10],
    target_buffer: &mut [Color],
    offset: usize,
    pattern: i32,
    dest_x: usize,
    dest_y: usize,
    source_width: usize,
) {
    let mut set_target_pixel = |x: usize, y: usize, c: Color| {
        let index = (y * source_width * 4) + x;
        target_buffer[index - offset] = c;
    };

    macro_rules! pixel00_0 {
        () => {
            set_target_pixel(dest_x, dest_y, w[5]);
        };
    }
    macro_rules! pixel00_11 {
        () => {
            set_target_pixel(dest_x, dest_y, interp1(w[5], w[4]));
        };
    }
    macro_rules! pixel00_12 {
        () => {
            set_target_pixel(dest_x, dest_y, interp1(w[5], w[2]));
        };
    }
    macro_rules! pixel00_20 {
        () => {
            set_target_pixel(dest_x, dest_y, interp2(w[5], w[2], w[4]));
        };
    }
    macro_rules! pixel00_50 {
        () => {
            set_target_pixel(dest_x, dest_y, interp5(w[2], w[4]));
        };
    }
    macro_rules! pixel00_80 {
        () => {
            set_target_pixel(dest_x, dest_y, interp8(w[5], w[1]));
        };
    }
    macro_rules! pixel00_81 {
        () => {
            set_target_pixel(dest_x, dest_y, interp8(w[5], w[4]));
        };
    }
    macro_rules! pixel00_82 {
        () => {
            set_target_pixel(dest_x, dest_y, interp8(w[5], w[2]));
        };
    }
    macro_rules! pixel01_0 {
        () => {
            set_target_pixel(dest_x + 1, dest_y, w[5]);
        };
    }
    macro_rules! pixel01_10 {
        () => {
            set_target_pixel(dest_x + 1, dest_y, interp1(w[5], w[1]));
        };
    }
    macro_rules! pixel01_12 {
        () => {
            set_target_pixel(dest_x + 1, dest_y, interp1(w[5], w[2]));
        };
    }
    macro_rules! pixel01_14 {
        () => {
            set_target_pixel(dest_x + 1, dest_y, interp1(w[2], w[5]));
        };
    }
    macro_rules! pixel01_21 {
        () => {
            set_target_pixel(dest_x + 1, dest_y, interp2(w[2], w[5], w[4]));
        };
    }
    macro_rules! pixel01_31 {
        () => {
            set_target_pixel(dest_x + 1, dest_y, interp3(w[5], w[4]));
        };
    }
    macro_rules! pixel01_50 {
        () => {
            set_target_pixel(dest_x + 1, dest_y, interp5(w[2], w[5]));
        };
    }
    macro_rules! pixel01_60 {
        () => {
            set_target_pixel(dest_x + 1, dest_y, interp6(w[5], w[2], w[4]));
        };
    }
    macro_rules! pixel01_61 {
        () => {
            set_target_pixel(dest_x + 1, dest_y, interp6(w[5], w[2], w[1]));
        };
    }
    macro_rules! pixel01_82 {
        () => {
            set_target_pixel(dest_x + 1, dest_y, interp8(w[5], w[2]));
        };
    }
    macro_rules! pixel01_83 {
        () => {
            set_target_pixel(dest_x + 1, dest_y, interp8(w[2], w[4]));
        };
    }
    macro_rules! pixel02_0 {
        () => {
            set_target_pixel(dest_x + 2, dest_y, w[5]);
        };
    }
    macro_rules! pixel02_10 {
        () => {
            set_target_pixel(dest_x + 2, dest_y, interp1(w[5], w[3]));
        };
    }
    macro_rules! pixel02_11 {
        () => {
            set_target_pixel(dest_x + 2, dest_y, interp1(w[5], w[2]));
        };
    }
    macro_rules! pixel02_13 {
        () => {
            set_target_pixel(dest_x + 2, dest_y, interp1(w[2], w[5]));
        };
    }
    macro_rules! pixel02_21 {
        () => {
            set_target_pixel(dest_x + 2, dest_y, interp2(w[2], w[5], w[6]));
        };
    }
    macro_rules! pixel02_32 {
        () => {
            set_target_pixel(dest_x + 2, dest_y, interp3(w[5], w[6]));
        };
    }
    macro_rules! pixel02_50 {
        () => {
            set_target_pixel(dest_x + 2, dest_y, interp5(w[2], w[5]));
        };
    }
    macro_rules! pixel02_60 {
        () => {
            set_target_pixel(dest_x + 2, dest_y, interp6(w[5], w[2], w[6]));
        };
    }
    macro_rules! pixel02_61 {
        () => {
            set_target_pixel(dest_x + 2, dest_y, interp6(w[5], w[2], w[3]));
        };
    }
    macro_rules! pixel02_81 {
        () => {
            set_target_pixel(dest_x + 2, dest_y, interp8(w[5], w[2]));
        };
    }
    macro_rules! pixel02_83 {
        () => {
            set_target_pixel(dest_x + 2, dest_y, interp8(w[2], w[6]));
        };
    }
    macro_rules! pixel03_0 {
        () => {
            set_target_pixel(dest_x + 3, dest_y, w[5]);
        };
    }
    macro_rules! pixel03_11 {
        () => {
            set_target_pixel(dest_x + 3, dest_y, interp1(w[5], w[2]));
        };
    }
    macro_rules! pixel03_12 {
        () => {
            set_target_pixel(dest_x + 3, dest_y, interp1(w[5], w[6]));
        };
    }
    macro_rules! pixel03_20 {
        () => {
            set_target_pixel(dest_x + 3, dest_y, interp2(w[5], w[2], w[6]));
        };
    }
    macro_rules! pixel03_50 {
        () => {
            set_target_pixel(dest_x + 3, dest_y, interp5(w[2], w[6]));
        };
    }
    macro_rules! pixel03_80 {
        () => {
            set_target_pixel(dest_x + 3, dest_y, interp8(w[5], w[3]));
        };
    }
    macro_rules! pixel03_81 {
        () => {
            set_target_pixel(dest_x + 3, dest_y, interp8(w[5], w[2]));
        };
    }
    macro_rules! pixel03_82 {
        () => {
            set_target_pixel(dest_x + 3, dest_y, interp8(w[5], w[6]));
        };
    }
    macro_rules! pixel10_0 {
        () => {
            set_target_pixel(dest_x, dest_y + 1, w[5]);
        };
    }
    macro_rules! pixel10_10 {
        () => {
            set_target_pixel(dest_x, dest_y + 1, interp1(w[5], w[1]));
        };
    }
    macro_rules! pixel10_11 {
        () => {
            set_target_pixel(dest_x, dest_y + 1, interp1(w[5], w[4]));
        };
    }
    macro_rules! pixel10_13 {
        () => {
            set_target_pixel(dest_x, dest_y + 1, interp1(w[4], w[5]));
        };
    }
    macro_rules! pixel10_21 {
        () => {
            set_target_pixel(dest_x, dest_y + 1, interp2(w[4], w[5], w[2]));
        };
    }
    macro_rules! pixel10_32 {
        () => {
            set_target_pixel(dest_x, dest_y + 1, interp3(w[5], w[2]));
        };
    }
    macro_rules! pixel10_50 {
        () => {
            set_target_pixel(dest_x, dest_y + 1, interp5(w[4], w[5]));
        };
    }
    macro_rules! pixel10_60 {
        () => {
            set_target_pixel(dest_x, dest_y + 1, interp6(w[5], w[4], w[2]));
        };
    }
    macro_rules! pixel10_61 {
        () => {
            set_target_pixel(dest_x, dest_y + 1, interp6(w[5], w[4], w[1]));
        };
    }
    macro_rules! pixel10_81 {
        () => {
            set_target_pixel(dest_x, dest_y + 1, interp8(w[5], w[4]));
        };
    }
    macro_rules! pixel10_83 {
        () => {
            set_target_pixel(dest_x, dest_y + 1, interp8(w[4], w[2]));
        };
    }
    macro_rules! pixel11_0 {
        () => {
            set_target_pixel(dest_x + 1, dest_y + 1, w[5]);
        };
    }
    macro_rules! pixel11_30 {
        () => {
            set_target_pixel(dest_x + 1, dest_y + 1, interp3(w[5], w[1]));
        };
    }
    macro_rules! pixel11_31 {
        () => {
            set_target_pixel(dest_x + 1, dest_y + 1, interp3(w[5], w[4]));
        };
    }
    macro_rules! pixel11_32 {
        () => {
            set_target_pixel(dest_x + 1, dest_y + 1, interp3(w[5], w[2]));
        };
    }
    macro_rules! pixel11_70 {
        () => {
            set_target_pixel(dest_x + 1, dest_y + 1, interp7(w[5], w[4], w[2]));
        };
    }
    macro_rules! pixel12_0 {
        () => {
            set_target_pixel(dest_x + 2, dest_y + 1, w[5]);
        };
    }
    macro_rules! pixel12_30 {
        () => {
            set_target_pixel(dest_x + 2, dest_y + 1, interp3(w[5], w[3]));
        };
    }
    macro_rules! pixel12_31 {
        () => {
            set_target_pixel(dest_x + 2, dest_y + 1, interp3(w[5], w[2]));
        };
    }
    macro_rules! pixel12_32 {
        () => {
            set_target_pixel(dest_x + 2, dest_y + 1, interp3(w[5], w[6]));
        };
    }
    macro_rules! pixel12_70 {
        () => {
            set_target_pixel(dest_x + 2, dest_y + 1, interp7(w[5], w[6], w[2]));
        };
    }
    macro_rules! pixel13_0 {
        () => {
            set_target_pixel(dest_x + 3, dest_y + 1, w[5]);
        };
    }
    macro_rules! pixel13_10 {
        () => {
            set_target_pixel(dest_x + 3, dest_y + 1, interp1(w[5], w[3]));
        };
    }
    macro_rules! pixel13_12 {
        () => {
            set_target_pixel(dest_x + 3, dest_y + 1, interp1(w[5], w[6]));
        };
    }
    macro_rules! pixel13_14 {
        () => {
            set_target_pixel(dest_x + 3, dest_y + 1, interp1(w[6], w[5]));
        };
    }
    macro_rules! pixel13_21 {
        () => {
            set_target_pixel(dest_x + 3, dest_y + 1, interp2(w[6], w[5], w[2]));
        };
    }
    macro_rules! pixel13_31 {
        () => {
            set_target_pixel(dest_x + 3, dest_y + 1, interp3(w[5], w[2]));
        };
    }
    macro_rules! pixel13_50 {
        () => {
            set_target_pixel(dest_x + 3, dest_y + 1, interp5(w[6], w[5]));
        };
    }
    macro_rules! pixel13_60 {
        () => {
            set_target_pixel(dest_x + 3, dest_y + 1, interp6(w[5], w[6], w[2]));
        };
    }
    macro_rules! pixel13_61 {
        () => {
            set_target_pixel(dest_x + 3, dest_y + 1, interp6(w[5], w[6], w[3]));
        };
    }
    macro_rules! pixel13_82 {
        () => {
            set_target_pixel(dest_x + 3, dest_y + 1, interp8(w[5], w[6]));
        };
    }
    macro_rules! pixel13_83 {
        () => {
            set_target_pixel(dest_x + 3, dest_y + 1, interp8(w[6], w[2]));
        };
    }
    macro_rules! pixel20_0 {
        () => {
            set_target_pixel(dest_x, dest_y + 2, w[5]);
        };
    }
    macro_rules! pixel20_10 {
        () => {
            set_target_pixel(dest_x, dest_y + 2, interp1(w[5], w[7]));
        };
    }
    macro_rules! pixel20_12 {
        () => {
            set_target_pixel(dest_x, dest_y + 2, interp1(w[5], w[4]));
        };
    }
    macro_rules! pixel20_14 {
        () => {
            set_target_pixel(dest_x, dest_y + 2, interp1(w[4], w[5]));
        };
    }
    macro_rules! pixel20_21 {
        () => {
            set_target_pixel(dest_x, dest_y + 2, interp2(w[4], w[5], w[8]));
        };
    }
    macro_rules! pixel20_31 {
        () => {
            set_target_pixel(dest_x, dest_y + 2, interp3(w[5], w[8]));
        };
    }
    macro_rules! pixel20_50 {
        () => {
            set_target_pixel(dest_x, dest_y + 2, interp5(w[4], w[5]));
        };
    }
    macro_rules! pixel20_60 {
        () => {
            set_target_pixel(dest_x, dest_y + 2, interp6(w[5], w[4], w[8]));
        };
    }
    macro_rules! pixel20_61 {
        () => {
            set_target_pixel(dest_x, dest_y + 2, interp6(w[5], w[4], w[7]));
        };
    }
    macro_rules! pixel20_82 {
        () => {
            set_target_pixel(dest_x, dest_y + 2, interp8(w[5], w[4]));
        };
    }
    macro_rules! pixel20_83 {
        () => {
            set_target_pixel(dest_x, dest_y + 2, interp8(w[4], w[8]));
        };
    }
    macro_rules! pixel21_0 {
        () => {
            set_target_pixel(dest_x + 1, dest_y + 2, w[5]);
        };
    }
    macro_rules! pixel21_30 {
        () => {
            set_target_pixel(dest_x + 1, dest_y + 2, interp3(w[5], w[7]));
        };
    }
    macro_rules! pixel21_31 {
        () => {
            set_target_pixel(dest_x + 1, dest_y + 2, interp3(w[5], w[8]));
        };
    }
    macro_rules! pixel21_32 {
        () => {
            set_target_pixel(dest_x + 1, dest_y + 2, interp3(w[5], w[4]));
        };
    }
    macro_rules! pixel21_70 {
        () => {
            set_target_pixel(dest_x + 1, dest_y + 2, interp7(w[5], w[4], w[8]));
        };
    }
    macro_rules! pixel22_0 {
        () => {
            set_target_pixel(dest_x + 2, dest_y + 2, w[5]);
        };
    }
    macro_rules! pixel22_30 {
        () => {
            set_target_pixel(dest_x + 2, dest_y + 2, interp3(w[5], w[9]));
        };
    }
    macro_rules! pixel22_31 {
        () => {
            set_target_pixel(dest_x + 2, dest_y + 2, interp3(w[5], w[6]));
        };
    }
    macro_rules! pixel22_32 {
        () => {
            set_target_pixel(dest_x + 2, dest_y + 2, interp3(w[5], w[8]));
        };
    }
    macro_rules! pixel22_70 {
        () => {
            set_target_pixel(dest_x + 2, dest_y + 2, interp7(w[5], w[6], w[8]));
        };
    }
    macro_rules! pixel23_0 {
        () => {
            set_target_pixel(dest_x + 3, dest_y + 2, w[5]);
        };
    }
    macro_rules! pixel23_10 {
        () => {
            set_target_pixel(dest_x + 3, dest_y + 2, interp1(w[5], w[9]));
        };
    }
    macro_rules! pixel23_11 {
        () => {
            set_target_pixel(dest_x + 3, dest_y + 2, interp1(w[5], w[6]));
        };
    }
    macro_rules! pixel23_13 {
        () => {
            set_target_pixel(dest_x + 3, dest_y + 2, interp1(w[6], w[5]));
        };
    }
    macro_rules! pixel23_21 {
        () => {
            set_target_pixel(dest_x + 3, dest_y + 2, interp2(w[6], w[5], w[8]));
        };
    }
    macro_rules! pixel23_32 {
        () => {
            set_target_pixel(dest_x + 3, dest_y + 2, interp3(w[5], w[8]));
        };
    }
    macro_rules! pixel23_50 {
        () => {
            set_target_pixel(dest_x + 3, dest_y + 2, interp5(w[6], w[5]));
        };
    }
    macro_rules! pixel23_60 {
        () => {
            set_target_pixel(dest_x + 3, dest_y + 2, interp6(w[5], w[6], w[8]));
        };
    }
    macro_rules! pixel23_61 {
        () => {
            set_target_pixel(dest_x + 3, dest_y + 2, interp6(w[5], w[6], w[9]));
        };
    }
    macro_rules! pixel23_81 {
        () => {
            set_target_pixel(dest_x + 3, dest_y + 2, interp8(w[5], w[6]));
        };
    }
    macro_rules! pixel23_83 {
        () => {
            set_target_pixel(dest_x + 3, dest_y + 2, interp8(w[6], w[8]));
        };
    }
    macro_rules! pixel30_0 {
        () => {
            set_target_pixel(dest_x, dest_y + 3, w[5]);
        };
    }
    macro_rules! pixel30_11 {
        () => {
            set_target_pixel(dest_x, dest_y + 3, interp1(w[5], w[8]));
        };
    }
    macro_rules! pixel30_12 {
        () => {
            set_target_pixel(dest_x, dest_y + 3, interp1(w[5], w[4]));
        };
    }
    macro_rules! pixel30_20 {
        () => {
            set_target_pixel(dest_x, dest_y + 3, interp2(w[5], w[8], w[4]));
        };
    }
    macro_rules! pixel30_50 {
        () => {
            set_target_pixel(dest_x, dest_y + 3, interp5(w[8], w[4]));
        };
    }
    macro_rules! pixel30_80 {
        () => {
            set_target_pixel(dest_x, dest_y + 3, interp8(w[5], w[7]));
        };
    }
    macro_rules! pixel30_81 {
        () => {
            set_target_pixel(dest_x, dest_y + 3, interp8(w[5], w[8]));
        };
    }
    macro_rules! pixel30_82 {
        () => {
            set_target_pixel(dest_x, dest_y + 3, interp8(w[5], w[4]));
        };
    }
    macro_rules! pixel31_0 {
        () => {
            set_target_pixel(dest_x + 1, dest_y + 3, w[5]);
        };
    }
    macro_rules! pixel31_10 {
        () => {
            set_target_pixel(dest_x + 1, dest_y + 3, interp1(w[5], w[7]));
        };
    }
    macro_rules! pixel31_11 {
        () => {
            set_target_pixel(dest_x + 1, dest_y + 3, interp1(w[5], w[8]));
        };
    }
    macro_rules! pixel31_13 {
        () => {
            set_target_pixel(dest_x + 1, dest_y + 3, interp1(w[8], w[5]));
        };
    }
    macro_rules! pixel31_21 {
        () => {
            set_target_pixel(dest_x + 1, dest_y + 3, interp2(w[8], w[5], w[4]));
        };
    }
    macro_rules! pixel31_32 {
        () => {
            set_target_pixel(dest_x + 1, dest_y + 3, interp3(w[5], w[4]));
        };
    }
    macro_rules! pixel31_50 {
        () => {
            set_target_pixel(dest_x + 1, dest_y + 3, interp5(w[8], w[5]));
        };
    }
    macro_rules! pixel31_60 {
        () => {
            set_target_pixel(dest_x + 1, dest_y + 3, interp6(w[5], w[8], w[4]));
        };
    }
    macro_rules! pixel31_61 {
        () => {
            set_target_pixel(dest_x + 1, dest_y + 3, interp6(w[5], w[8], w[7]));
        };
    }
    macro_rules! pixel31_81 {
        () => {
            set_target_pixel(dest_x + 1, dest_y + 3, interp8(w[5], w[8]));
        };
    }
    macro_rules! pixel31_83 {
        () => {
            set_target_pixel(dest_x + 1, dest_y + 3, interp8(w[8], w[4]));
        };
    }
    macro_rules! pixel32_0 {
        () => {
            set_target_pixel(dest_x + 2, dest_y + 3, w[5]);
        };
    }
    macro_rules! pixel32_10 {
        () => {
            set_target_pixel(dest_x + 2, dest_y + 3, interp1(w[5], w[9]));
        };
    }
    macro_rules! pixel32_12 {
        () => {
            set_target_pixel(dest_x + 2, dest_y + 3, interp1(w[5], w[8]));
        };
    }
    macro_rules! pixel32_14 {
        () => {
            set_target_pixel(dest_x + 2, dest_y + 3, interp1(w[8], w[5]));
        };
    }
    macro_rules! pixel32_21 {
        () => {
            set_target_pixel(dest_x + 2, dest_y + 3, interp2(w[8], w[5], w[6]));
        };
    }
    macro_rules! pixel32_31 {
        () => {
            set_target_pixel(dest_x + 2, dest_y + 3, interp3(w[5], w[6]));
        };
    }
    macro_rules! pixel32_50 {
        () => {
            set_target_pixel(dest_x + 2, dest_y + 3, interp5(w[8], w[5]));
        };
    }
    macro_rules! pixel32_60 {
        () => {
            set_target_pixel(dest_x + 2, dest_y + 3, interp6(w[5], w[8], w[6]));
        };
    }
    macro_rules! pixel32_61 {
        () => {
            set_target_pixel(dest_x + 2, dest_y + 3, interp6(w[5], w[8], w[9]));
        };
    }
    macro_rules! pixel32_82 {
        () => {
            set_target_pixel(dest_x + 2, dest_y + 3, interp8(w[5], w[8]));
        };
    }
    macro_rules! pixel32_83 {
        () => {
            set_target_pixel(dest_x + 2, dest_y + 3, interp8(w[8], w[6]));
        };
    }
    macro_rules! pixel33_0 {
        () => {
            set_target_pixel(dest_x + 3, dest_y + 3, w[5]);
        };
    }
    macro_rules! pixel33_11 {
        () => {
            set_target_pixel(dest_x + 3, dest_y + 3, interp1(w[5], w[6]));
        };
    }
    macro_rules! pixel33_12 {
        () => {
            set_target_pixel(dest_x + 3, dest_y + 3, interp1(w[5], w[8]));
        };
    }
    macro_rules! pixel33_20 {
        () => {
            set_target_pixel(dest_x + 3, dest_y + 3, interp2(w[5], w[8], w[6]));
        };
    }
    macro_rules! pixel33_50 {
        () => {
            set_target_pixel(dest_x + 3, dest_y + 3, interp5(w[8], w[6]));
        };
    }
    macro_rules! pixel33_80 {
        () => {
            set_target_pixel(dest_x + 3, dest_y + 3, interp8(w[5], w[9]));
        };
    }
    macro_rules! pixel33_81 {
        () => {
            set_target_pixel(dest_x + 3, dest_y + 3, interp8(w[5], w[6]));
        };
    }
    macro_rules! pixel33_82 {
        () => {
            set_target_pixel(dest_x + 3, dest_y + 3, interp8(w[5], w[8]));
        };
    }

    match pattern {
        0 | 1 | 4 | 32 | 128 | 5 | 132 | 160 | 33 | 129 | 36 | 133 | 164 | 161 | 37 | 165 => {
            pixel00_20!();
            pixel01_60!();
            pixel02_60!();
            pixel03_20!();
            pixel10_60!();
            pixel11_70!();
            pixel12_70!();
            pixel13_60!();
            pixel20_60!();
            pixel21_70!();
            pixel22_70!();
            pixel23_60!();
            pixel30_20!();
            pixel31_60!();
            pixel32_60!();
            pixel33_20!();
        }
        2 | 34 | 130 | 162 => {
            pixel00_80!();
            pixel01_10!();
            pixel02_10!();
            pixel03_80!();
            pixel10_61!();
            pixel11_30!();
            pixel12_30!();
            pixel13_61!();
            pixel20_60!();
            pixel21_70!();
            pixel22_70!();
            pixel23_60!();
            pixel30_20!();
            pixel31_60!();
            pixel32_60!();
            pixel33_20!();
        }
        16 | 17 | 48 | 49 => {
            pixel00_20!();
            pixel01_60!();
            pixel02_61!();
            pixel03_80!();
            pixel10_60!();
            pixel11_70!();
            pixel12_30!();
            pixel13_10!();
            pixel20_60!();
            pixel21_70!();
            pixel22_30!();
            pixel23_10!();
            pixel30_20!();
            pixel31_60!();
            pixel32_61!();
            pixel33_80!();
        }
        64 | 65 | 68 | 69 => {
            pixel00_20!();
            pixel01_60!();
            pixel02_60!();
            pixel03_20!();
            pixel10_60!();
            pixel11_70!();
            pixel12_70!();
            pixel13_60!();
            pixel20_61!();
            pixel21_30!();
            pixel22_30!();
            pixel23_61!();
            pixel30_80!();
            pixel31_10!();
            pixel32_10!();
            pixel33_80!();
        }
        8 | 12 | 136 | 140 => {
            pixel00_80!();
            pixel01_61!();
            pixel02_60!();
            pixel03_20!();
            pixel10_10!();
            pixel11_30!();
            pixel12_70!();
            pixel13_60!();
            pixel20_10!();
            pixel21_30!();
            pixel22_70!();
            pixel23_60!();
            pixel30_80!();
            pixel31_61!();
            pixel32_60!();
            pixel33_20!();
        }
        3 | 35 | 131 | 163 => {
            pixel00_81!();
            pixel01_31!();
            pixel02_10!();
            pixel03_80!();
            pixel10_81!();
            pixel11_31!();
            pixel12_30!();
            pixel13_61!();
            pixel20_60!();
            pixel21_70!();
            pixel22_70!();
            pixel23_60!();
            pixel30_20!();
            pixel31_60!();
            pixel32_60!();
            pixel33_20!();
        }
        6 | 38 | 134 | 166 => {
            pixel00_80!();
            pixel01_10!();
            pixel02_32!();
            pixel03_82!();
            pixel10_61!();
            pixel11_30!();
            pixel12_32!();
            pixel13_82!();
            pixel20_60!();
            pixel21_70!();
            pixel22_70!();
            pixel23_60!();
            pixel30_20!();
            pixel31_60!();
            pixel32_60!();
            pixel33_20!();
        }
        20 | 21 | 52 | 53 => {
            pixel00_20!();
            pixel01_60!();
            pixel02_81!();
            pixel03_81!();
            pixel10_60!();
            pixel11_70!();
            pixel12_31!();
            pixel13_31!();
            pixel20_60!();
            pixel21_70!();
            pixel22_30!();
            pixel23_10!();
            pixel30_20!();
            pixel31_60!();
            pixel32_61!();
            pixel33_80!();
        }
        144 | 145 | 176 | 177 => {
            pixel00_20!();
            pixel01_60!();
            pixel02_61!();
            pixel03_80!();
            pixel10_60!();
            pixel11_70!();
            pixel12_30!();
            pixel13_10!();
            pixel20_60!();
            pixel21_70!();
            pixel22_32!();
            pixel23_32!();
            pixel30_20!();
            pixel31_60!();
            pixel32_82!();
            pixel33_82!();
        }
        192 | 193 | 196 | 197 => {
            pixel00_20!();
            pixel01_60!();
            pixel02_60!();
            pixel03_20!();
            pixel10_60!();
            pixel11_70!();
            pixel12_70!();
            pixel13_60!();
            pixel20_61!();
            pixel21_30!();
            pixel22_31!();
            pixel23_81!();
            pixel30_80!();
            pixel31_10!();
            pixel32_31!();
            pixel33_81!();
        }
        96 | 97 | 100 | 101 => {
            pixel00_20!();
            pixel01_60!();
            pixel02_60!();
            pixel03_20!();
            pixel10_60!();
            pixel11_70!();
            pixel12_70!();
            pixel13_60!();
            pixel20_82!();
            pixel21_32!();
            pixel22_30!();
            pixel23_61!();
            pixel30_82!();
            pixel31_32!();
            pixel32_10!();
            pixel33_80!();
        }
        40 | 44 | 168 | 172 => {
            pixel00_80!();
            pixel01_61!();
            pixel02_60!();
            pixel03_20!();
            pixel10_10!();
            pixel11_30!();
            pixel12_70!();
            pixel13_60!();
            pixel20_31!();
            pixel21_31!();
            pixel22_70!();
            pixel23_60!();
            pixel30_81!();
            pixel31_81!();
            pixel32_60!();
            pixel33_20!();
        }
        9 | 13 | 137 | 141 => {
            pixel00_82!();
            pixel01_82!();
            pixel02_60!();
            pixel03_20!();
            pixel10_32!();
            pixel11_32!();
            pixel12_70!();
            pixel13_60!();
            pixel20_10!();
            pixel21_30!();
            pixel22_70!();
            pixel23_60!();
            pixel30_80!();
            pixel31_61!();
            pixel32_60!();
            pixel33_20!();
        }
        18 | 50 => {
            pixel00_80!();
            pixel01_10!();
            if color_diff(w[2], w[6]) {
                pixel02_10!();
                pixel03_80!();
                pixel12_30!();
                pixel13_10!();
            } else {
                pixel02_50!();
                pixel03_50!();
                pixel12_0!();
                pixel13_50!();
            }
            pixel10_61!();
            pixel11_30!();
            pixel20_60!();
            pixel21_70!();
            pixel22_30!();
            pixel23_10!();
            pixel30_20!();
            pixel31_60!();
            pixel32_61!();
            pixel33_80!();
        }
        80 | 81 => {
            pixel00_20!();
            pixel01_60!();
            pixel02_61!();
            pixel03_80!();
            pixel10_60!();
            pixel11_70!();
            pixel12_30!();
            pixel13_10!();
            pixel20_61!();
            pixel21_30!();
            if color_diff(w[6], w[8]) {
                pixel22_30!();
                pixel23_10!();
                pixel32_10!();
                pixel33_80!();
            } else {
                pixel22_0!();
                pixel23_50!();
                pixel32_50!();
                pixel33_50!();
            }
            pixel30_80!();
            pixel31_10!();
        }
        72 | 76 => {
            pixel00_80!();
            pixel01_61!();
            pixel02_60!();
            pixel03_20!();
            pixel10_10!();
            pixel11_30!();
            pixel12_70!();
            pixel13_60!();
            if color_diff(w[8], w[4]) {
                pixel20_10!();
                pixel21_30!();
                pixel30_80!();
                pixel31_10!();
            } else {
                pixel20_50!();
                pixel21_0!();
                pixel30_50!();
                pixel31_50!();
            }
            pixel22_30!();
            pixel23_61!();
            pixel32_10!();
            pixel33_80!();
        }
        10 | 138 => {
            if color_diff(w[4], w[2]) {
                pixel00_80!();
                pixel01_10!();
                pixel10_10!();
                pixel11_30!();
            } else {
                pixel00_50!();
                pixel01_50!();
                pixel10_50!();
                pixel11_0!();
            }
            pixel02_10!();
            pixel03_80!();
            pixel12_30!();
            pixel13_61!();
            pixel20_10!();
            pixel21_30!();
            pixel22_70!();
            pixel23_60!();
            pixel30_80!();
            pixel31_61!();
            pixel32_60!();
            pixel33_20!();
        }
        66 => {
            pixel00_80!();
            pixel01_10!();
            pixel02_10!();
            pixel03_80!();
            pixel10_61!();
            pixel11_30!();
            pixel12_30!();
            pixel13_61!();
            pixel20_61!();
            pixel21_30!();
            pixel22_30!();
            pixel23_61!();
            pixel30_80!();
            pixel31_10!();
            pixel32_10!();
            pixel33_80!();
        }
        24 => {
            pixel00_80!();
            pixel01_61!();
            pixel02_61!();
            pixel03_80!();
            pixel10_10!();
            pixel11_30!();
            pixel12_30!();
            pixel13_10!();
            pixel20_10!();
            pixel21_30!();
            pixel22_30!();
            pixel23_10!();
            pixel30_80!();
            pixel31_61!();
            pixel32_61!();
            pixel33_80!();
        }
        7 | 39 | 135 => {
            pixel00_81!();
            pixel01_31!();
            pixel02_32!();
            pixel03_82!();
            pixel10_81!();
            pixel11_31!();
            pixel12_32!();
            pixel13_82!();
            pixel20_60!();
            pixel21_70!();
            pixel22_70!();
            pixel23_60!();
            pixel30_20!();
            pixel31_60!();
            pixel32_60!();
            pixel33_20!();
        }
        148 | 149 | 180 => {
            pixel00_20!();
            pixel01_60!();
            pixel02_81!();
            pixel03_81!();
            pixel10_60!();
            pixel11_70!();
            pixel12_31!();
            pixel13_31!();
            pixel20_60!();
            pixel21_70!();
            pixel22_32!();
            pixel23_32!();
            pixel30_20!();
            pixel31_60!();
            pixel32_82!();
            pixel33_82!();
        }
        224 | 228 | 225 => {
            pixel00_20!();
            pixel01_60!();
            pixel02_60!();
            pixel03_20!();
            pixel10_60!();
            pixel11_70!();
            pixel12_70!();
            pixel13_60!();
            pixel20_82!();
            pixel21_32!();
            pixel22_31!();
            pixel23_81!();
            pixel30_82!();
            pixel31_32!();
            pixel32_31!();
            pixel33_81!();
        }
        41 | 169 | 45 => {
            pixel00_82!();
            pixel01_82!();
            pixel02_60!();
            pixel03_20!();
            pixel10_32!();
            pixel11_32!();
            pixel12_70!();
            pixel13_60!();
            pixel20_31!();
            pixel21_31!();
            pixel22_70!();
            pixel23_60!();
            pixel30_81!();
            pixel31_81!();
            pixel32_60!();
            pixel33_20!();
        }
        22 | 54 => {
            pixel00_80!();
            pixel01_10!();
            if color_diff(w[2], w[6]) {
                pixel02_0!();
                pixel03_0!();
                pixel13_0!();
            } else {
                pixel02_50!();
                pixel03_50!();
                pixel13_50!();
            }
            pixel10_61!();
            pixel11_30!();
            pixel12_0!();
            pixel20_60!();
            pixel21_70!();
            pixel22_30!();
            pixel23_10!();
            pixel30_20!();
            pixel31_60!();
            pixel32_61!();
            pixel33_80!();
        }
        208 | 209 => {
            pixel00_20!();
            pixel01_60!();
            pixel02_61!();
            pixel03_80!();
            pixel10_60!();
            pixel11_70!();
            pixel12_30!();
            pixel13_10!();
            pixel20_61!();
            pixel21_30!();
            pixel22_0!();
            if color_diff(w[6], w[8]) {
                pixel23_0!();
                pixel32_0!();
                pixel33_0!();
            } else {
                pixel23_50!();
                pixel32_50!();
                pixel33_50!();
            }
            pixel30_80!();
            pixel31_10!();
        }
        104 | 108 => {
            pixel00_80!();
            pixel01_61!();
            pixel02_60!();
            pixel03_20!();
            pixel10_10!();
            pixel11_30!();
            pixel12_70!();
            pixel13_60!();
            if color_diff(w[8], w[4]) {
                pixel20_0!();
                pixel30_0!();
                pixel31_0!();
            } else {
                pixel20_50!();
                pixel30_50!();
                pixel31_50!();
            }
            pixel21_0!();
            pixel22_30!();
            pixel23_61!();
            pixel32_10!();
            pixel33_80!();
        }
        11 | 139 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
                pixel01_0!();
                pixel10_0!();
            } else {
                pixel00_50!();
                pixel01_50!();
                pixel10_50!();
            }
            pixel02_10!();
            pixel03_80!();
            pixel11_0!();
            pixel12_30!();
            pixel13_61!();
            pixel20_10!();
            pixel21_30!();
            pixel22_70!();
            pixel23_60!();
            pixel30_80!();
            pixel31_61!();
            pixel32_60!();
            pixel33_20!();
        }
        19 | 51 => {
            if color_diff(w[2], w[6]) {
                pixel00_81!();
                pixel01_31!();
                pixel02_10!();
                pixel03_80!();
                pixel12_30!();
                pixel13_10!();
            } else {
                pixel00_12!();
                pixel01_14!();
                pixel02_83!();
                pixel03_50!();
                pixel12_70!();
                pixel13_21!();
            }
            pixel10_81!();
            pixel11_31!();
            pixel20_60!();
            pixel21_70!();
            pixel22_30!();
            pixel23_10!();
            pixel30_20!();
            pixel31_60!();
            pixel32_61!();
            pixel33_80!();
        }
        146 | 178 => {
            pixel00_80!();
            pixel01_10!();
            if color_diff(w[2], w[6]) {
                pixel02_10!();
                pixel03_80!();
                pixel12_30!();
                pixel13_10!();
                pixel23_32!();
                pixel33_82!();
            } else {
                pixel02_21!();
                pixel03_50!();
                pixel12_70!();
                pixel13_83!();
                pixel23_13!();
                pixel33_11!();
            }
            pixel10_61!();
            pixel11_30!();
            pixel20_60!();
            pixel21_70!();
            pixel22_32!();
            pixel30_20!();
            pixel31_60!();
            pixel32_82!();
        }
        84 | 85 => {
            pixel00_20!();
            pixel01_60!();
            pixel02_81!();
            if color_diff(w[6], w[8]) {
                pixel03_81!();
                pixel13_31!();
                pixel22_30!();
                pixel23_10!();
                pixel32_10!();
                pixel33_80!();
            } else {
                pixel03_12!();
                pixel13_14!();
                pixel22_70!();
                pixel23_83!();
                pixel32_21!();
                pixel33_50!();
            }
            pixel10_60!();
            pixel11_70!();
            pixel12_31!();
            pixel20_61!();
            pixel21_30!();
            pixel30_80!();
            pixel31_10!();
        }
        112 | 113 => {
            pixel00_20!();
            pixel01_60!();
            pixel02_61!();
            pixel03_80!();
            pixel10_60!();
            pixel11_70!();
            pixel12_30!();
            pixel13_10!();
            pixel20_82!();
            pixel21_32!();
            if color_diff(w[6], w[8]) {
                pixel22_30!();
                pixel23_10!();
                pixel30_82!();
                pixel31_32!();
                pixel32_10!();
                pixel33_80!();
            } else {
                pixel22_70!();
                pixel23_21!();
                pixel30_11!();
                pixel31_13!();
                pixel32_83!();
                pixel33_50!();
            }
        }
        200 | 204 => {
            pixel00_80!();
            pixel01_61!();
            pixel02_60!();
            pixel03_20!();
            pixel10_10!();
            pixel11_30!();
            pixel12_70!();
            pixel13_60!();
            if color_diff(w[8], w[4]) {
                pixel20_10!();
                pixel21_30!();
                pixel30_80!();
                pixel31_10!();
                pixel32_31!();
                pixel33_81!();
            } else {
                pixel20_21!();
                pixel21_70!();
                pixel30_50!();
                pixel31_83!();
                pixel32_14!();
                pixel33_12!();
            }
            pixel22_31!();
            pixel23_81!();
        }
        73 | 77 => {
            if color_diff(w[8], w[4]) {
                pixel00_82!();
                pixel10_32!();
                pixel20_10!();
                pixel21_30!();
                pixel30_80!();
                pixel31_10!();
            } else {
                pixel00_11!();
                pixel10_13!();
                pixel20_83!();
                pixel21_70!();
                pixel30_50!();
                pixel31_21!();
            }
            pixel01_82!();
            pixel02_60!();
            pixel03_20!();
            pixel11_32!();
            pixel12_70!();
            pixel13_60!();
            pixel22_30!();
            pixel23_61!();
            pixel32_10!();
            pixel33_80!();
        }
        42 | 170 => {
            if color_diff(w[4], w[2]) {
                pixel00_80!();
                pixel01_10!();
                pixel10_10!();
                pixel11_30!();
                pixel20_31!();
                pixel30_81!();
            } else {
                pixel00_50!();
                pixel01_21!();
                pixel10_83!();
                pixel11_70!();
                pixel20_14!();
                pixel30_12!();
            }
            pixel02_10!();
            pixel03_80!();
            pixel12_30!();
            pixel13_61!();
            pixel21_31!();
            pixel22_70!();
            pixel23_60!();
            pixel31_81!();
            pixel32_60!();
            pixel33_20!();
        }
        14 | 142 => {
            if color_diff(w[4], w[2]) {
                pixel00_80!();
                pixel01_10!();
                pixel02_32!();
                pixel03_82!();
                pixel10_10!();
                pixel11_30!();
            } else {
                pixel00_50!();
                pixel01_83!();
                pixel02_13!();
                pixel03_11!();
                pixel10_21!();
                pixel11_70!();
            }
            pixel12_32!();
            pixel13_82!();
            pixel20_10!();
            pixel21_30!();
            pixel22_70!();
            pixel23_60!();
            pixel30_80!();
            pixel31_61!();
            pixel32_60!();
            pixel33_20!();
        }
        67 => {
            pixel00_81!();
            pixel01_31!();
            pixel02_10!();
            pixel03_80!();
            pixel10_81!();
            pixel11_31!();
            pixel12_30!();
            pixel13_61!();
            pixel20_61!();
            pixel21_30!();
            pixel22_30!();
            pixel23_61!();
            pixel30_80!();
            pixel31_10!();
            pixel32_10!();
            pixel33_80!();
        }
        70 => {
            pixel00_80!();
            pixel01_10!();
            pixel02_32!();
            pixel03_82!();
            pixel10_61!();
            pixel11_30!();
            pixel12_32!();
            pixel13_82!();
            pixel20_61!();
            pixel21_30!();
            pixel22_30!();
            pixel23_61!();
            pixel30_80!();
            pixel31_10!();
            pixel32_10!();
            pixel33_80!();
        }
        28 => {
            pixel00_80!();
            pixel01_61!();
            pixel02_81!();
            pixel03_81!();
            pixel10_10!();
            pixel11_30!();
            pixel12_31!();
            pixel13_31!();
            pixel20_10!();
            pixel21_30!();
            pixel22_30!();
            pixel23_10!();
            pixel30_80!();
            pixel31_61!();
            pixel32_61!();
            pixel33_80!();
        }
        152 => {
            pixel00_80!();
            pixel01_61!();
            pixel02_61!();
            pixel03_80!();
            pixel10_10!();
            pixel11_30!();
            pixel12_30!();
            pixel13_10!();
            pixel20_10!();
            pixel21_30!();
            pixel22_32!();
            pixel23_32!();
            pixel30_80!();
            pixel31_61!();
            pixel32_82!();
            pixel33_82!();
        }
        194 => {
            pixel00_80!();
            pixel01_10!();
            pixel02_10!();
            pixel03_80!();
            pixel10_61!();
            pixel11_30!();
            pixel12_30!();
            pixel13_61!();
            pixel20_61!();
            pixel21_30!();
            pixel22_31!();
            pixel23_81!();
            pixel30_80!();
            pixel31_10!();
            pixel32_31!();
            pixel33_81!();
        }
        98 => {
            pixel00_80!();
            pixel01_10!();
            pixel02_10!();
            pixel03_80!();
            pixel10_61!();
            pixel11_30!();
            pixel12_30!();
            pixel13_61!();
            pixel20_82!();
            pixel21_32!();
            pixel22_30!();
            pixel23_61!();
            pixel30_82!();
            pixel31_32!();
            pixel32_10!();
            pixel33_80!();
        }
        56 => {
            pixel00_80!();
            pixel01_61!();
            pixel02_61!();
            pixel03_80!();
            pixel10_10!();
            pixel11_30!();
            pixel12_30!();
            pixel13_10!();
            pixel20_31!();
            pixel21_31!();
            pixel22_30!();
            pixel23_10!();
            pixel30_81!();
            pixel31_81!();
            pixel32_61!();
            pixel33_80!();
        }
        25 => {
            pixel00_82!();
            pixel01_82!();
            pixel02_61!();
            pixel03_80!();
            pixel10_32!();
            pixel11_32!();
            pixel12_30!();
            pixel13_10!();
            pixel20_10!();
            pixel21_30!();
            pixel22_30!();
            pixel23_10!();
            pixel30_80!();
            pixel31_61!();
            pixel32_61!();
            pixel33_80!();
        }
        26 | 31 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
                pixel01_0!();
                pixel10_0!();
            } else {
                pixel00_50!();
                pixel01_50!();
                pixel10_50!();
            }
            if color_diff(w[2], w[6]) {
                pixel02_0!();
                pixel03_0!();
                pixel13_0!();
            } else {
                pixel02_50!();
                pixel03_50!();
                pixel13_50!();
            }
            pixel11_0!();
            pixel12_0!();
            pixel20_10!();
            pixel21_30!();
            pixel22_30!();
            pixel23_10!();
            pixel30_80!();
            pixel31_61!();
            pixel32_61!();
            pixel33_80!();
        }
        82 | 214 => {
            pixel00_80!();
            pixel01_10!();
            if color_diff(w[2], w[6]) {
                pixel02_0!();
                pixel03_0!();
                pixel13_0!();
            } else {
                pixel02_50!();
                pixel03_50!();
                pixel13_50!();
            }
            pixel10_61!();
            pixel11_30!();
            pixel12_0!();
            pixel20_61!();
            pixel21_30!();
            pixel22_0!();
            if color_diff(w[6], w[8]) {
                pixel23_0!();
                pixel32_0!();
                pixel33_0!();
            } else {
                pixel23_50!();
                pixel32_50!();
                pixel33_50!();
            }
            pixel30_80!();
            pixel31_10!();
        }
        88 | 248 => {
            pixel00_80!();
            pixel01_61!();
            pixel02_61!();
            pixel03_80!();
            pixel10_10!();
            pixel11_30!();
            pixel12_30!();
            pixel13_10!();
            if color_diff(w[8], w[4]) {
                pixel20_0!();
                pixel30_0!();
                pixel31_0!();
            } else {
                pixel20_50!();
                pixel30_50!();
                pixel31_50!();
            }
            pixel21_0!();
            pixel22_0!();
            if color_diff(w[6], w[8]) {
                pixel23_0!();
                pixel32_0!();
                pixel33_0!();
            } else {
                pixel23_50!();
                pixel32_50!();
                pixel33_50!();
            }
        }
        74 | 107 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
                pixel01_0!();
                pixel10_0!();
            } else {
                pixel00_50!();
                pixel01_50!();
                pixel10_50!();
            }
            pixel02_10!();
            pixel03_80!();
            pixel11_0!();
            pixel12_30!();
            pixel13_61!();
            if color_diff(w[8], w[4]) {
                pixel20_0!();
                pixel30_0!();
                pixel31_0!();
            } else {
                pixel20_50!();
                pixel30_50!();
                pixel31_50!();
            }
            pixel21_0!();
            pixel22_30!();
            pixel23_61!();
            pixel32_10!();
            pixel33_80!();
        }
        27 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
                pixel01_0!();
                pixel10_0!();
            } else {
                pixel00_50!();
                pixel01_50!();
                pixel10_50!();
            }
            pixel02_10!();
            pixel03_80!();
            pixel11_0!();
            pixel12_30!();
            pixel13_10!();
            pixel20_10!();
            pixel21_30!();
            pixel22_30!();
            pixel23_10!();
            pixel30_80!();
            pixel31_61!();
            pixel32_61!();
            pixel33_80!();
        }
        86 => {
            pixel00_80!();
            pixel01_10!();
            if color_diff(w[2], w[6]) {
                pixel02_0!();
                pixel03_0!();
                pixel13_0!();
            } else {
                pixel02_50!();
                pixel03_50!();
                pixel13_50!();
            }
            pixel10_61!();
            pixel11_30!();
            pixel12_0!();
            pixel20_61!();
            pixel21_30!();
            pixel22_30!();
            pixel23_10!();
            pixel30_80!();
            pixel31_10!();
            pixel32_10!();
            pixel33_80!();
        }
        216 => {
            pixel00_80!();
            pixel01_61!();
            pixel02_61!();
            pixel03_80!();
            pixel10_10!();
            pixel11_30!();
            pixel12_30!();
            pixel13_10!();
            pixel20_10!();
            pixel21_30!();
            pixel22_0!();
            if color_diff(w[6], w[8]) {
                pixel23_0!();
                pixel32_0!();
                pixel33_0!();
            } else {
                pixel23_50!();
                pixel32_50!();
                pixel33_50!();
            }
            pixel30_80!();
            pixel31_10!();
        }
        106 => {
            pixel00_80!();
            pixel01_10!();
            pixel02_10!();
            pixel03_80!();
            pixel10_10!();
            pixel11_30!();
            pixel12_30!();
            pixel13_61!();
            if color_diff(w[8], w[4]) {
                pixel20_0!();
                pixel30_0!();
                pixel31_0!();
            } else {
                pixel20_50!();
                pixel30_50!();
                pixel31_50!();
            }
            pixel21_0!();
            pixel22_30!();
            pixel23_61!();
            pixel32_10!();
            pixel33_80!();
        }
        30 => {
            pixel00_80!();
            pixel01_10!();
            if color_diff(w[2], w[6]) {
                pixel02_0!();
                pixel03_0!();
                pixel13_0!();
            } else {
                pixel02_50!();
                pixel03_50!();
                pixel13_50!();
            }
            pixel10_10!();
            pixel11_30!();
            pixel12_0!();
            pixel20_10!();
            pixel21_30!();
            pixel22_30!();
            pixel23_10!();
            pixel30_80!();
            pixel31_61!();
            pixel32_61!();
            pixel33_80!();
        }
        210 => {
            pixel00_80!();
            pixel01_10!();
            pixel02_10!();
            pixel03_80!();
            pixel10_61!();
            pixel11_30!();
            pixel12_30!();
            pixel13_10!();
            pixel20_61!();
            pixel21_30!();
            pixel22_0!();
            if color_diff(w[6], w[8]) {
                pixel23_0!();
                pixel32_0!();
                pixel33_0!();
            } else {
                pixel23_50!();
                pixel32_50!();
                pixel33_50!();
            }
            pixel30_80!();
            pixel31_10!();
        }
        120 => {
            pixel00_80!();
            pixel01_61!();
            pixel02_61!();
            pixel03_80!();
            pixel10_10!();
            pixel11_30!();
            pixel12_30!();
            pixel13_10!();
            if color_diff(w[8], w[4]) {
                pixel20_0!();
                pixel30_0!();
                pixel31_0!();
            } else {
                pixel20_50!();
                pixel30_50!();
                pixel31_50!();
            }
            pixel21_0!();
            pixel22_30!();
            pixel23_10!();
            pixel32_10!();
            pixel33_80!();
        }
        75 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
                pixel01_0!();
                pixel10_0!();
            } else {
                pixel00_50!();
                pixel01_50!();
                pixel10_50!();
            }
            pixel02_10!();
            pixel03_80!();
            pixel11_0!();
            pixel12_30!();
            pixel13_61!();
            pixel20_10!();
            pixel21_30!();
            pixel22_30!();
            pixel23_61!();
            pixel30_80!();
            pixel31_10!();
            pixel32_10!();
            pixel33_80!();
        }
        29 => {
            pixel00_82!();
            pixel01_82!();
            pixel02_81!();
            pixel03_81!();
            pixel10_32!();
            pixel11_32!();
            pixel12_31!();
            pixel13_31!();
            pixel20_10!();
            pixel21_30!();
            pixel22_30!();
            pixel23_10!();
            pixel30_80!();
            pixel31_61!();
            pixel32_61!();
            pixel33_80!();
        }
        198 => {
            pixel00_80!();
            pixel01_10!();
            pixel02_32!();
            pixel03_82!();
            pixel10_61!();
            pixel11_30!();
            pixel12_32!();
            pixel13_82!();
            pixel20_61!();
            pixel21_30!();
            pixel22_31!();
            pixel23_81!();
            pixel30_80!();
            pixel31_10!();
            pixel32_31!();
            pixel33_81!();
        }
        184 => {
            pixel00_80!();
            pixel01_61!();
            pixel02_61!();
            pixel03_80!();
            pixel10_10!();
            pixel11_30!();
            pixel12_30!();
            pixel13_10!();
            pixel20_31!();
            pixel21_31!();
            pixel22_32!();
            pixel23_32!();
            pixel30_81!();
            pixel31_81!();
            pixel32_82!();
            pixel33_82!();
        }
        99 => {
            pixel00_81!();
            pixel01_31!();
            pixel02_10!();
            pixel03_80!();
            pixel10_81!();
            pixel11_31!();
            pixel12_30!();
            pixel13_61!();
            pixel20_82!();
            pixel21_32!();
            pixel22_30!();
            pixel23_61!();
            pixel30_82!();
            pixel31_32!();
            pixel32_10!();
            pixel33_80!();
        }
        57 => {
            pixel00_82!();
            pixel01_82!();
            pixel02_61!();
            pixel03_80!();
            pixel10_32!();
            pixel11_32!();
            pixel12_30!();
            pixel13_10!();
            pixel20_31!();
            pixel21_31!();
            pixel22_30!();
            pixel23_10!();
            pixel30_81!();
            pixel31_81!();
            pixel32_61!();
            pixel33_80!();
        }
        71 => {
            pixel00_81!();
            pixel01_31!();
            pixel02_32!();
            pixel03_82!();
            pixel10_81!();
            pixel11_31!();
            pixel12_32!();
            pixel13_82!();
            pixel20_61!();
            pixel21_30!();
            pixel22_30!();
            pixel23_61!();
            pixel30_80!();
            pixel31_10!();
            pixel32_10!();
            pixel33_80!();
        }
        156 => {
            pixel00_80!();
            pixel01_61!();
            pixel02_81!();
            pixel03_81!();
            pixel10_10!();
            pixel11_30!();
            pixel12_31!();
            pixel13_31!();
            pixel20_10!();
            pixel21_30!();
            pixel22_32!();
            pixel23_32!();
            pixel30_80!();
            pixel31_61!();
            pixel32_82!();
            pixel33_82!();
        }
        226 => {
            pixel00_80!();
            pixel01_10!();
            pixel02_10!();
            pixel03_80!();
            pixel10_61!();
            pixel11_30!();
            pixel12_30!();
            pixel13_61!();
            pixel20_82!();
            pixel21_32!();
            pixel22_31!();
            pixel23_81!();
            pixel30_82!();
            pixel31_32!();
            pixel32_31!();
            pixel33_81!();
        }
        60 => {
            pixel00_80!();
            pixel01_61!();
            pixel02_81!();
            pixel03_81!();
            pixel10_10!();
            pixel11_30!();
            pixel12_31!();
            pixel13_31!();
            pixel20_31!();
            pixel21_31!();
            pixel22_30!();
            pixel23_10!();
            pixel30_81!();
            pixel31_81!();
            pixel32_61!();
            pixel33_80!();
        }
        195 => {
            pixel00_81!();
            pixel01_31!();
            pixel02_10!();
            pixel03_80!();
            pixel10_81!();
            pixel11_31!();
            pixel12_30!();
            pixel13_61!();
            pixel20_61!();
            pixel21_30!();
            pixel22_31!();
            pixel23_81!();
            pixel30_80!();
            pixel31_10!();
            pixel32_31!();
            pixel33_81!();
        }
        102 => {
            pixel00_80!();
            pixel01_10!();
            pixel02_32!();
            pixel03_82!();
            pixel10_61!();
            pixel11_30!();
            pixel12_32!();
            pixel13_82!();
            pixel20_82!();
            pixel21_32!();
            pixel22_30!();
            pixel23_61!();
            pixel30_82!();
            pixel31_32!();
            pixel32_10!();
            pixel33_80!();
        }
        153 => {
            pixel00_82!();
            pixel01_82!();
            pixel02_61!();
            pixel03_80!();
            pixel10_32!();
            pixel11_32!();
            pixel12_30!();
            pixel13_10!();
            pixel20_10!();
            pixel21_30!();
            pixel22_32!();
            pixel23_32!();
            pixel30_80!();
            pixel31_61!();
            pixel32_82!();
            pixel33_82!();
        }
        58 => {
            if color_diff(w[4], w[2]) {
                pixel00_80!();
                pixel01_10!();
                pixel10_10!();
                pixel11_30!();
            } else {
                pixel00_20!();
                pixel01_12!();
                pixel10_11!();
                pixel11_0!();
            }
            if color_diff(w[2], w[6]) {
                pixel02_10!();
                pixel03_80!();
                pixel12_30!();
                pixel13_10!();
            } else {
                pixel02_11!();
                pixel03_20!();
                pixel12_0!();
                pixel13_12!();
            }
            pixel20_31!();
            pixel21_31!();
            pixel22_30!();
            pixel23_10!();
            pixel30_81!();
            pixel31_81!();
            pixel32_61!();
            pixel33_80!();
        }
        83 => {
            pixel00_81!();
            pixel01_31!();
            if color_diff(w[2], w[6]) {
                pixel02_10!();
                pixel03_80!();
                pixel12_30!();
                pixel13_10!();
            } else {
                pixel02_11!();
                pixel03_20!();
                pixel12_0!();
                pixel13_12!();
            }
            pixel10_81!();
            pixel11_31!();
            pixel20_61!();
            pixel21_30!();
            if color_diff(w[6], w[8]) {
                pixel22_30!();
                pixel23_10!();
                pixel32_10!();
                pixel33_80!();
            } else {
                pixel22_0!();
                pixel23_11!();
                pixel32_12!();
                pixel33_20!();
            }
            pixel30_80!();
            pixel31_10!();
        }
        92 => {
            pixel00_80!();
            pixel01_61!();
            pixel02_81!();
            pixel03_81!();
            pixel10_10!();
            pixel11_30!();
            pixel12_31!();
            pixel13_31!();
            if color_diff(w[8], w[4]) {
                pixel20_10!();
                pixel21_30!();
                pixel30_80!();
                pixel31_10!();
            } else {
                pixel20_12!();
                pixel21_0!();
                pixel30_20!();
                pixel31_11!();
            }
            if color_diff(w[6], w[8]) {
                pixel22_30!();
                pixel23_10!();
                pixel32_10!();
                pixel33_80!();
            } else {
                pixel22_0!();
                pixel23_11!();
                pixel32_12!();
                pixel33_20!();
            }
        }
        202 => {
            if color_diff(w[4], w[2]) {
                pixel00_80!();
                pixel01_10!();
                pixel10_10!();
                pixel11_30!();
            } else {
                pixel00_20!();
                pixel01_12!();
                pixel10_11!();
                pixel11_0!();
            }
            pixel02_10!();
            pixel03_80!();
            pixel12_30!();
            pixel13_61!();
            if color_diff(w[8], w[4]) {
                pixel20_10!();
                pixel21_30!();
                pixel30_80!();
                pixel31_10!();
            } else {
                pixel20_12!();
                pixel21_0!();
                pixel30_20!();
                pixel31_11!();
            }
            pixel22_31!();
            pixel23_81!();
            pixel32_31!();
            pixel33_81!();
        }
        78 => {
            if color_diff(w[4], w[2]) {
                pixel00_80!();
                pixel01_10!();
                pixel10_10!();
                pixel11_30!();
            } else {
                pixel00_20!();
                pixel01_12!();
                pixel10_11!();
                pixel11_0!();
            }
            pixel02_32!();
            pixel03_82!();
            pixel12_32!();
            pixel13_82!();
            if color_diff(w[8], w[4]) {
                pixel20_10!();
                pixel21_30!();
                pixel30_80!();
                pixel31_10!();
            } else {
                pixel20_12!();
                pixel21_0!();
                pixel30_20!();
                pixel31_11!();
            }
            pixel22_30!();
            pixel23_61!();
            pixel32_10!();
            pixel33_80!();
        }
        154 => {
            if color_diff(w[4], w[2]) {
                pixel00_80!();
                pixel01_10!();
                pixel10_10!();
                pixel11_30!();
            } else {
                pixel00_20!();
                pixel01_12!();
                pixel10_11!();
                pixel11_0!();
            }
            if color_diff(w[2], w[6]) {
                pixel02_10!();
                pixel03_80!();
                pixel12_30!();
                pixel13_10!();
            } else {
                pixel02_11!();
                pixel03_20!();
                pixel12_0!();
                pixel13_12!();
            }
            pixel20_10!();
            pixel21_30!();
            pixel22_32!();
            pixel23_32!();
            pixel30_80!();
            pixel31_61!();
            pixel32_82!();
            pixel33_82!();
        }
        114 => {
            pixel00_80!();
            pixel01_10!();
            if color_diff(w[2], w[6]) {
                pixel02_10!();
                pixel03_80!();
                pixel12_30!();
                pixel13_10!();
            } else {
                pixel02_11!();
                pixel03_20!();
                pixel12_0!();
                pixel13_12!();
            }
            pixel10_61!();
            pixel11_30!();
            pixel20_82!();
            pixel21_32!();
            if color_diff(w[6], w[8]) {
                pixel22_30!();
                pixel23_10!();
                pixel32_10!();
                pixel33_80!();
            } else {
                pixel22_0!();
                pixel23_11!();
                pixel32_12!();
                pixel33_20!();
            }
            pixel30_82!();
            pixel31_32!();
        }
        89 => {
            pixel00_82!();
            pixel01_82!();
            pixel02_61!();
            pixel03_80!();
            pixel10_32!();
            pixel11_32!();
            pixel12_30!();
            pixel13_10!();
            if color_diff(w[8], w[4]) {
                pixel20_10!();
                pixel21_30!();
                pixel30_80!();
                pixel31_10!();
            } else {
                pixel20_12!();
                pixel21_0!();
                pixel30_20!();
                pixel31_11!();
            }
            if color_diff(w[6], w[8]) {
                pixel22_30!();
                pixel23_10!();
                pixel32_10!();
                pixel33_80!();
            } else {
                pixel22_0!();
                pixel23_11!();
                pixel32_12!();
                pixel33_20!();
            }
        }
        90 => {
            if color_diff(w[4], w[2]) {
                pixel00_80!();
                pixel01_10!();
                pixel10_10!();
                pixel11_30!();
            } else {
                pixel00_20!();
                pixel01_12!();
                pixel10_11!();
                pixel11_0!();
            }
            if color_diff(w[2], w[6]) {
                pixel02_10!();
                pixel03_80!();
                pixel12_30!();
                pixel13_10!();
            } else {
                pixel02_11!();
                pixel03_20!();
                pixel12_0!();
                pixel13_12!();
            }
            if color_diff(w[8], w[4]) {
                pixel20_10!();
                pixel21_30!();
                pixel30_80!();
                pixel31_10!();
            } else {
                pixel20_12!();
                pixel21_0!();
                pixel30_20!();
                pixel31_11!();
            }
            if color_diff(w[6], w[8]) {
                pixel22_30!();
                pixel23_10!();
                pixel32_10!();
                pixel33_80!();
            } else {
                pixel22_0!();
                pixel23_11!();
                pixel32_12!();
                pixel33_20!();
            }
        }
        55 | 23 => {
            if color_diff(w[2], w[6]) {
                pixel00_81!();
                pixel01_31!();
                pixel02_0!();
                pixel03_0!();
                pixel12_0!();
                pixel13_0!();
            } else {
                pixel00_12!();
                pixel01_14!();
                pixel02_83!();
                pixel03_50!();
                pixel12_70!();
                pixel13_21!();
            }
            pixel10_81!();
            pixel11_31!();
            pixel20_60!();
            pixel21_70!();
            pixel22_30!();
            pixel23_10!();
            pixel30_20!();
            pixel31_60!();
            pixel32_61!();
            pixel33_80!();
        }
        182 | 150 => {
            pixel00_80!();
            pixel01_10!();
            if color_diff(w[2], w[6]) {
                pixel02_0!();
                pixel03_0!();
                pixel12_0!();
                pixel13_0!();
                pixel23_32!();
                pixel33_82!();
            } else {
                pixel02_21!();
                pixel03_50!();
                pixel12_70!();
                pixel13_83!();
                pixel23_13!();
                pixel33_11!();
            }
            pixel10_61!();
            pixel11_30!();
            pixel20_60!();
            pixel21_70!();
            pixel22_32!();
            pixel30_20!();
            pixel31_60!();
            pixel32_82!();
        }
        213 | 212 => {
            pixel00_20!();
            pixel01_60!();
            pixel02_81!();
            if color_diff(w[6], w[8]) {
                pixel03_81!();
                pixel13_31!();
                pixel22_0!();
                pixel23_0!();
                pixel32_0!();
                pixel33_0!();
            } else {
                pixel03_12!();
                pixel13_14!();
                pixel22_70!();
                pixel23_83!();
                pixel32_21!();
                pixel33_50!();
            }
            pixel10_60!();
            pixel11_70!();
            pixel12_31!();
            pixel20_61!();
            pixel21_30!();
            pixel30_80!();
            pixel31_10!();
        }
        241 | 240 => {
            pixel00_20!();
            pixel01_60!();
            pixel02_61!();
            pixel03_80!();
            pixel10_60!();
            pixel11_70!();
            pixel12_30!();
            pixel13_10!();
            pixel20_82!();
            pixel21_32!();
            if color_diff(w[6], w[8]) {
                pixel22_0!();
                pixel23_0!();
                pixel30_82!();
                pixel31_32!();
                pixel32_0!();
                pixel33_0!();
            } else {
                pixel22_70!();
                pixel23_21!();
                pixel30_11!();
                pixel31_13!();
                pixel32_83!();
                pixel33_50!();
            }
        }
        236 | 232 => {
            pixel00_80!();
            pixel01_61!();
            pixel02_60!();
            pixel03_20!();
            pixel10_10!();
            pixel11_30!();
            pixel12_70!();
            pixel13_60!();
            if color_diff(w[8], w[4]) {
                pixel20_0!();
                pixel21_0!();
                pixel30_0!();
                pixel31_0!();
                pixel32_31!();
                pixel33_81!();
            } else {
                pixel20_21!();
                pixel21_70!();
                pixel30_50!();
                pixel31_83!();
                pixel32_14!();
                pixel33_12!();
            }
            pixel22_31!();
            pixel23_81!();
        }
        109 | 105 => {
            if color_diff(w[8], w[4]) {
                pixel00_82!();
                pixel10_32!();
                pixel20_0!();
                pixel21_0!();
                pixel30_0!();
                pixel31_0!();
            } else {
                pixel00_11!();
                pixel10_13!();
                pixel20_83!();
                pixel21_70!();
                pixel30_50!();
                pixel31_21!();
            }
            pixel01_82!();
            pixel02_60!();
            pixel03_20!();
            pixel11_32!();
            pixel12_70!();
            pixel13_60!();
            pixel22_30!();
            pixel23_61!();
            pixel32_10!();
            pixel33_80!();
        }
        171 | 43 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
                pixel01_0!();
                pixel10_0!();
                pixel11_0!();
                pixel20_31!();
                pixel30_81!();
            } else {
                pixel00_50!();
                pixel01_21!();
                pixel10_83!();
                pixel11_70!();
                pixel20_14!();
                pixel30_12!();
            }
            pixel02_10!();
            pixel03_80!();
            pixel12_30!();
            pixel13_61!();
            pixel21_31!();
            pixel22_70!();
            pixel23_60!();
            pixel31_81!();
            pixel32_60!();
            pixel33_20!();
        }
        143 | 15 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
                pixel01_0!();
                pixel02_32!();
                pixel03_82!();
                pixel10_0!();
                pixel11_0!();
            } else {
                pixel00_50!();
                pixel01_83!();
                pixel02_13!();
                pixel03_11!();
                pixel10_21!();
                pixel11_70!();
            }
            pixel12_32!();
            pixel13_82!();
            pixel20_10!();
            pixel21_30!();
            pixel22_70!();
            pixel23_60!();
            pixel30_80!();
            pixel31_61!();
            pixel32_60!();
            pixel33_20!();
        }
        124 => {
            pixel00_80!();
            pixel01_61!();
            pixel02_81!();
            pixel03_81!();
            pixel10_10!();
            pixel11_30!();
            pixel12_31!();
            pixel13_31!();
            if color_diff(w[8], w[4]) {
                pixel20_0!();
                pixel30_0!();
                pixel31_0!();
            } else {
                pixel20_50!();
                pixel30_50!();
                pixel31_50!();
            }
            pixel21_0!();
            pixel22_30!();
            pixel23_10!();
            pixel32_10!();
            pixel33_80!();
        }
        203 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
                pixel01_0!();
                pixel10_0!();
            } else {
                pixel00_50!();
                pixel01_50!();
                pixel10_50!();
            }
            pixel02_10!();
            pixel03_80!();
            pixel11_0!();
            pixel12_30!();
            pixel13_61!();
            pixel20_10!();
            pixel21_30!();
            pixel22_31!();
            pixel23_81!();
            pixel30_80!();
            pixel31_10!();
            pixel32_31!();
            pixel33_81!();
        }
        62 => {
            pixel00_80!();
            pixel01_10!();
            if color_diff(w[2], w[6]) {
                pixel02_0!();
                pixel03_0!();
                pixel13_0!();
            } else {
                pixel02_50!();
                pixel03_50!();
                pixel13_50!();
            }
            pixel10_10!();
            pixel11_30!();
            pixel12_0!();
            pixel20_31!();
            pixel21_31!();
            pixel22_30!();
            pixel23_10!();
            pixel30_81!();
            pixel31_81!();
            pixel32_61!();
            pixel33_80!();
        }
        211 => {
            pixel00_81!();
            pixel01_31!();
            pixel02_10!();
            pixel03_80!();
            pixel10_81!();
            pixel11_31!();
            pixel12_30!();
            pixel13_10!();
            pixel20_61!();
            pixel21_30!();
            pixel22_0!();
            if color_diff(w[6], w[8]) {
                pixel23_0!();
                pixel32_0!();
                pixel33_0!();
            } else {
                pixel23_50!();
                pixel32_50!();
                pixel33_50!();
            }
            pixel30_80!();
            pixel31_10!();
        }
        118 => {
            pixel00_80!();
            pixel01_10!();
            if color_diff(w[2], w[6]) {
                pixel02_0!();
                pixel03_0!();
                pixel13_0!();
            } else {
                pixel02_50!();
                pixel03_50!();
                pixel13_50!();
            }
            pixel10_61!();
            pixel11_30!();
            pixel12_0!();
            pixel20_82!();
            pixel21_32!();
            pixel22_30!();
            pixel23_10!();
            pixel30_82!();
            pixel31_32!();
            pixel32_10!();
            pixel33_80!();
        }
        217 => {
            pixel00_82!();
            pixel01_82!();
            pixel02_61!();
            pixel03_80!();
            pixel10_32!();
            pixel11_32!();
            pixel12_30!();
            pixel13_10!();
            pixel20_10!();
            pixel21_30!();
            pixel22_0!();
            if color_diff(w[6], w[8]) {
                pixel23_0!();
                pixel32_0!();
                pixel33_0!();
            } else {
                pixel23_50!();
                pixel32_50!();
                pixel33_50!();
            }
            pixel30_80!();
            pixel31_10!();
        }
        110 => {
            pixel00_80!();
            pixel01_10!();
            pixel02_32!();
            pixel03_82!();
            pixel10_10!();
            pixel11_30!();
            pixel12_32!();
            pixel13_82!();
            if color_diff(w[8], w[4]) {
                pixel20_0!();
                pixel30_0!();
                pixel31_0!();
            } else {
                pixel20_50!();
                pixel30_50!();
                pixel31_50!();
            }
            pixel21_0!();
            pixel22_30!();
            pixel23_61!();
            pixel32_10!();
            pixel33_80!();
        }
        155 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
                pixel01_0!();
                pixel10_0!();
            } else {
                pixel00_50!();
                pixel01_50!();
                pixel10_50!();
            }
            pixel02_10!();
            pixel03_80!();
            pixel11_0!();
            pixel12_30!();
            pixel13_10!();
            pixel20_10!();
            pixel21_30!();
            pixel22_32!();
            pixel23_32!();
            pixel30_80!();
            pixel31_61!();
            pixel32_82!();
            pixel33_82!();
        }
        188 => {
            pixel00_80!();
            pixel01_61!();
            pixel02_81!();
            pixel03_81!();
            pixel10_10!();
            pixel11_30!();
            pixel12_31!();
            pixel13_31!();
            pixel20_31!();
            pixel21_31!();
            pixel22_32!();
            pixel23_32!();
            pixel30_81!();
            pixel31_81!();
            pixel32_82!();
            pixel33_82!();
        }
        185 => {
            pixel00_82!();
            pixel01_82!();
            pixel02_61!();
            pixel03_80!();
            pixel10_32!();
            pixel11_32!();
            pixel12_30!();
            pixel13_10!();
            pixel20_31!();
            pixel21_31!();
            pixel22_32!();
            pixel23_32!();
            pixel30_81!();
            pixel31_81!();
            pixel32_82!();
            pixel33_82!();
        }
        61 => {
            pixel00_82!();
            pixel01_82!();
            pixel02_81!();
            pixel03_81!();
            pixel10_32!();
            pixel11_32!();
            pixel12_31!();
            pixel13_31!();
            pixel20_31!();
            pixel21_31!();
            pixel22_30!();
            pixel23_10!();
            pixel30_81!();
            pixel31_81!();
            pixel32_61!();
            pixel33_80!();
        }
        157 => {
            pixel00_82!();
            pixel01_82!();
            pixel02_81!();
            pixel03_81!();
            pixel10_32!();
            pixel11_32!();
            pixel12_31!();
            pixel13_31!();
            pixel20_10!();
            pixel21_30!();
            pixel22_32!();
            pixel23_32!();
            pixel30_80!();
            pixel31_61!();
            pixel32_82!();
            pixel33_82!();
        }
        103 => {
            pixel00_81!();
            pixel01_31!();
            pixel02_32!();
            pixel03_82!();
            pixel10_81!();
            pixel11_31!();
            pixel12_32!();
            pixel13_82!();
            pixel20_82!();
            pixel21_32!();
            pixel22_30!();
            pixel23_61!();
            pixel30_82!();
            pixel31_32!();
            pixel32_10!();
            pixel33_80!();
        }
        227 => {
            pixel00_81!();
            pixel01_31!();
            pixel02_10!();
            pixel03_80!();
            pixel10_81!();
            pixel11_31!();
            pixel12_30!();
            pixel13_61!();
            pixel20_82!();
            pixel21_32!();
            pixel22_31!();
            pixel23_81!();
            pixel30_82!();
            pixel31_32!();
            pixel32_31!();
            pixel33_81!();
        }
        230 => {
            pixel00_80!();
            pixel01_10!();
            pixel02_32!();
            pixel03_82!();
            pixel10_61!();
            pixel11_30!();
            pixel12_32!();
            pixel13_82!();
            pixel20_82!();
            pixel21_32!();
            pixel22_31!();
            pixel23_81!();
            pixel30_82!();
            pixel31_32!();
            pixel32_31!();
            pixel33_81!();
        }
        199 => {
            pixel00_81!();
            pixel01_31!();
            pixel02_32!();
            pixel03_82!();
            pixel10_81!();
            pixel11_31!();
            pixel12_32!();
            pixel13_82!();
            pixel20_61!();
            pixel21_30!();
            pixel22_31!();
            pixel23_81!();
            pixel30_80!();
            pixel31_10!();
            pixel32_31!();
            pixel33_81!();
        }
        220 => {
            pixel00_80!();
            pixel01_61!();
            pixel02_81!();
            pixel03_81!();
            pixel10_10!();
            pixel11_30!();
            pixel12_31!();
            pixel13_31!();
            if color_diff(w[8], w[4]) {
                pixel20_10!();
                pixel21_30!();
                pixel30_80!();
                pixel31_10!();
            } else {
                pixel20_12!();
                pixel21_0!();
                pixel30_20!();
                pixel31_11!();
            }
            pixel22_0!();
            if color_diff(w[6], w[8]) {
                pixel23_0!();
                pixel32_0!();
                pixel33_0!();
            } else {
                pixel23_50!();
                pixel32_50!();
                pixel33_50!();
            }
        }
        158 => {
            if color_diff(w[4], w[2]) {
                pixel00_80!();
                pixel01_10!();
                pixel10_10!();
                pixel11_30!();
            } else {
                pixel00_20!();
                pixel01_12!();
                pixel10_11!();
                pixel11_0!();
            }
            if color_diff(w[2], w[6]) {
                pixel02_0!();
                pixel03_0!();
                pixel13_0!();
            } else {
                pixel02_50!();
                pixel03_50!();
                pixel13_50!();
            }
            pixel12_0!();
            pixel20_10!();
            pixel21_30!();
            pixel22_32!();
            pixel23_32!();
            pixel30_80!();
            pixel31_61!();
            pixel32_82!();
            pixel33_82!();
        }
        234 => {
            if color_diff(w[4], w[2]) {
                pixel00_80!();
                pixel01_10!();
                pixel10_10!();
                pixel11_30!();
            } else {
                pixel00_20!();
                pixel01_12!();
                pixel10_11!();
                pixel11_0!();
            }
            pixel02_10!();
            pixel03_80!();
            pixel12_30!();
            pixel13_61!();
            if color_diff(w[8], w[4]) {
                pixel20_0!();
                pixel30_0!();
                pixel31_0!();
            } else {
                pixel20_50!();
                pixel30_50!();
                pixel31_50!();
            }
            pixel21_0!();
            pixel22_31!();
            pixel23_81!();
            pixel32_31!();
            pixel33_81!();
        }
        242 => {
            pixel00_80!();
            pixel01_10!();
            if color_diff(w[2], w[6]) {
                pixel02_10!();
                pixel03_80!();
                pixel12_30!();
                pixel13_10!();
            } else {
                pixel02_11!();
                pixel03_20!();
                pixel12_0!();
                pixel13_12!();
            }
            pixel10_61!();
            pixel11_30!();
            pixel20_82!();
            pixel21_32!();
            pixel22_0!();
            if color_diff(w[6], w[8]) {
                pixel23_0!();
                pixel32_0!();
                pixel33_0!();
            } else {
                pixel23_50!();
                pixel32_50!();
                pixel33_50!();
            }
            pixel30_82!();
            pixel31_32!();
        }
        59 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
                pixel01_0!();
                pixel10_0!();
            } else {
                pixel00_50!();
                pixel01_50!();
                pixel10_50!();
            }
            if color_diff(w[2], w[6]) {
                pixel02_10!();
                pixel03_80!();
                pixel12_30!();
                pixel13_10!();
            } else {
                pixel02_11!();
                pixel03_20!();
                pixel12_0!();
                pixel13_12!();
            }
            pixel11_0!();
            pixel20_31!();
            pixel21_31!();
            pixel22_30!();
            pixel23_10!();
            pixel30_81!();
            pixel31_81!();
            pixel32_61!();
            pixel33_80!();
        }
        121 => {
            pixel00_82!();
            pixel01_82!();
            pixel02_61!();
            pixel03_80!();
            pixel10_32!();
            pixel11_32!();
            pixel12_30!();
            pixel13_10!();
            if color_diff(w[8], w[4]) {
                pixel20_0!();
                pixel30_0!();
                pixel31_0!();
            } else {
                pixel20_50!();
                pixel30_50!();
                pixel31_50!();
            }
            pixel21_0!();
            if color_diff(w[6], w[8]) {
                pixel22_30!();
                pixel23_10!();
                pixel32_10!();
                pixel33_80!();
            } else {
                pixel22_0!();
                pixel23_11!();
                pixel32_12!();
                pixel33_20!();
            }
        }
        87 => {
            pixel00_81!();
            pixel01_31!();
            if color_diff(w[2], w[6]) {
                pixel02_0!();
                pixel03_0!();
                pixel13_0!();
            } else {
                pixel02_50!();
                pixel03_50!();
                pixel13_50!();
            }
            pixel10_81!();
            pixel11_31!();
            pixel12_0!();
            pixel20_61!();
            pixel21_30!();
            if color_diff(w[6], w[8]) {
                pixel22_30!();
                pixel23_10!();
                pixel32_10!();
                pixel33_80!();
            } else {
                pixel22_0!();
                pixel23_11!();
                pixel32_12!();
                pixel33_20!();
            }
            pixel30_80!();
            pixel31_10!();
        }
        79 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
                pixel01_0!();
                pixel10_0!();
            } else {
                pixel00_50!();
                pixel01_50!();
                pixel10_50!();
            }
            pixel02_32!();
            pixel03_82!();
            pixel11_0!();
            pixel12_32!();
            pixel13_82!();
            if color_diff(w[8], w[4]) {
                pixel20_10!();
                pixel21_30!();
                pixel30_80!();
                pixel31_10!();
            } else {
                pixel20_12!();
                pixel21_0!();
                pixel30_20!();
                pixel31_11!();
            }
            pixel22_30!();
            pixel23_61!();
            pixel32_10!();
            pixel33_80!();
        }
        122 => {
            if color_diff(w[4], w[2]) {
                pixel00_80!();
                pixel01_10!();
                pixel10_10!();
                pixel11_30!();
            } else {
                pixel00_20!();
                pixel01_12!();
                pixel10_11!();
                pixel11_0!();
            }
            if color_diff(w[2], w[6]) {
                pixel02_10!();
                pixel03_80!();
                pixel12_30!();
                pixel13_10!();
            } else {
                pixel02_11!();
                pixel03_20!();
                pixel12_0!();
                pixel13_12!();
            }
            if color_diff(w[8], w[4]) {
                pixel20_0!();
                pixel30_0!();
                pixel31_0!();
            } else {
                pixel20_50!();
                pixel30_50!();
                pixel31_50!();
            }
            pixel21_0!();
            if color_diff(w[6], w[8]) {
                pixel22_30!();
                pixel23_10!();
                pixel32_10!();
                pixel33_80!();
            } else {
                pixel22_0!();
                pixel23_11!();
                pixel32_12!();
                pixel33_20!();
            }
        }
        94 => {
            if color_diff(w[4], w[2]) {
                pixel00_80!();
                pixel01_10!();
                pixel10_10!();
                pixel11_30!();
            } else {
                pixel00_20!();
                pixel01_12!();
                pixel10_11!();
                pixel11_0!();
            }
            if color_diff(w[2], w[6]) {
                pixel02_0!();
                pixel03_0!();
                pixel13_0!();
            } else {
                pixel02_50!();
                pixel03_50!();
                pixel13_50!();
            }
            pixel12_0!();
            if color_diff(w[8], w[4]) {
                pixel20_10!();
                pixel21_30!();
                pixel30_80!();
                pixel31_10!();
            } else {
                pixel20_12!();
                pixel21_0!();
                pixel30_20!();
                pixel31_11!();
            }
            if color_diff(w[6], w[8]) {
                pixel22_30!();
                pixel23_10!();
                pixel32_10!();
                pixel33_80!();
            } else {
                pixel22_0!();
                pixel23_11!();
                pixel32_12!();
                pixel33_20!();
            }
        }
        218 => {
            if color_diff(w[4], w[2]) {
                pixel00_80!();
                pixel01_10!();
                pixel10_10!();
                pixel11_30!();
            } else {
                pixel00_20!();
                pixel01_12!();
                pixel10_11!();
                pixel11_0!();
            }
            if color_diff(w[2], w[6]) {
                pixel02_10!();
                pixel03_80!();
                pixel12_30!();
                pixel13_10!();
            } else {
                pixel02_11!();
                pixel03_20!();
                pixel12_0!();
                pixel13_12!();
            }
            if color_diff(w[8], w[4]) {
                pixel20_10!();
                pixel21_30!();
                pixel30_80!();
                pixel31_10!();
            } else {
                pixel20_12!();
                pixel21_0!();
                pixel30_20!();
                pixel31_11!();
            }
            pixel22_0!();
            if color_diff(w[6], w[8]) {
                pixel23_0!();
                pixel32_0!();
                pixel33_0!();
            } else {
                pixel23_50!();
                pixel32_50!();
                pixel33_50!();
            }
        }
        91 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
                pixel01_0!();
                pixel10_0!();
            } else {
                pixel00_50!();
                pixel01_50!();
                pixel10_50!();
            }
            if color_diff(w[2], w[6]) {
                pixel02_10!();
                pixel03_80!();
                pixel12_30!();
                pixel13_10!();
            } else {
                pixel02_11!();
                pixel03_20!();
                pixel12_0!();
                pixel13_12!();
            }
            pixel11_0!();
            if color_diff(w[8], w[4]) {
                pixel20_10!();
                pixel21_30!();
                pixel30_80!();
                pixel31_10!();
            } else {
                pixel20_12!();
                pixel21_0!();
                pixel30_20!();
                pixel31_11!();
            }
            if color_diff(w[6], w[8]) {
                pixel22_30!();
                pixel23_10!();
                pixel32_10!();
                pixel33_80!();
            } else {
                pixel22_0!();
                pixel23_11!();
                pixel32_12!();
                pixel33_20!();
            }
        }
        229 => {
            pixel00_20!();
            pixel01_60!();
            pixel02_60!();
            pixel03_20!();
            pixel10_60!();
            pixel11_70!();
            pixel12_70!();
            pixel13_60!();
            pixel20_82!();
            pixel21_32!();
            pixel22_31!();
            pixel23_81!();
            pixel30_82!();
            pixel31_32!();
            pixel32_31!();
            pixel33_81!();
        }
        167 => {
            pixel00_81!();
            pixel01_31!();
            pixel02_32!();
            pixel03_82!();
            pixel10_81!();
            pixel11_31!();
            pixel12_32!();
            pixel13_82!();
            pixel20_60!();
            pixel21_70!();
            pixel22_70!();
            pixel23_60!();
            pixel30_20!();
            pixel31_60!();
            pixel32_60!();
            pixel33_20!();
        }
        173 => {
            pixel00_82!();
            pixel01_82!();
            pixel02_60!();
            pixel03_20!();
            pixel10_32!();
            pixel11_32!();
            pixel12_70!();
            pixel13_60!();
            pixel20_31!();
            pixel21_31!();
            pixel22_70!();
            pixel23_60!();
            pixel30_81!();
            pixel31_81!();
            pixel32_60!();
            pixel33_20!();
        }
        181 => {
            pixel00_20!();
            pixel01_60!();
            pixel02_81!();
            pixel03_81!();
            pixel10_60!();
            pixel11_70!();
            pixel12_31!();
            pixel13_31!();
            pixel20_60!();
            pixel21_70!();
            pixel22_32!();
            pixel23_32!();
            pixel30_20!();
            pixel31_60!();
            pixel32_82!();
            pixel33_82!();
        }
        186 => {
            if color_diff(w[4], w[2]) {
                pixel00_80!();
                pixel01_10!();
                pixel10_10!();
                pixel11_30!();
            } else {
                pixel00_20!();
                pixel01_12!();
                pixel10_11!();
                pixel11_0!();
            }
            if color_diff(w[2], w[6]) {
                pixel02_10!();
                pixel03_80!();
                pixel12_30!();
                pixel13_10!();
            } else {
                pixel02_11!();
                pixel03_20!();
                pixel12_0!();
                pixel13_12!();
            }
            pixel20_31!();
            pixel21_31!();
            pixel22_32!();
            pixel23_32!();
            pixel30_81!();
            pixel31_81!();
            pixel32_82!();
            pixel33_82!();
        }
        115 => {
            pixel00_81!();
            pixel01_31!();
            if color_diff(w[2], w[6]) {
                pixel02_10!();
                pixel03_80!();
                pixel12_30!();
                pixel13_10!();
            } else {
                pixel02_11!();
                pixel03_20!();
                pixel12_0!();
                pixel13_12!();
            }
            pixel10_81!();
            pixel11_31!();
            pixel20_82!();
            pixel21_32!();
            if color_diff(w[6], w[8]) {
                pixel22_30!();
                pixel23_10!();
                pixel32_10!();
                pixel33_80!();
            } else {
                pixel22_0!();
                pixel23_11!();
                pixel32_12!();
                pixel33_20!();
            }
            pixel30_82!();
            pixel31_32!();
        }
        93 => {
            pixel00_82!();
            pixel01_82!();
            pixel02_81!();
            pixel03_81!();
            pixel10_32!();
            pixel11_32!();
            pixel12_31!();
            pixel13_31!();
            if color_diff(w[8], w[4]) {
                pixel20_10!();
                pixel21_30!();
                pixel30_80!();
                pixel31_10!();
            } else {
                pixel20_12!();
                pixel21_0!();
                pixel30_20!();
                pixel31_11!();
            }
            if color_diff(w[6], w[8]) {
                pixel22_30!();
                pixel23_10!();
                pixel32_10!();
                pixel33_80!();
            } else {
                pixel22_0!();
                pixel23_11!();
                pixel32_12!();
                pixel33_20!();
            }
        }
        206 => {
            if color_diff(w[4], w[2]) {
                pixel00_80!();
                pixel01_10!();
                pixel10_10!();
                pixel11_30!();
            } else {
                pixel00_20!();
                pixel01_12!();
                pixel10_11!();
                pixel11_0!();
            }
            pixel02_32!();
            pixel03_82!();
            pixel12_32!();
            pixel13_82!();
            if color_diff(w[8], w[4]) {
                pixel20_10!();
                pixel21_30!();
                pixel30_80!();
                pixel31_10!();
            } else {
                pixel20_12!();
                pixel21_0!();
                pixel30_20!();
                pixel31_11!();
            }
            pixel22_31!();
            pixel23_81!();
            pixel32_31!();
            pixel33_81!();
        }
        205 | 201 => {
            pixel00_82!();
            pixel01_82!();
            pixel02_60!();
            pixel03_20!();
            pixel10_32!();
            pixel11_32!();
            pixel12_70!();
            pixel13_60!();
            if color_diff(w[8], w[4]) {
                pixel20_10!();
                pixel21_30!();
                pixel30_80!();
                pixel31_10!();
            } else {
                pixel20_12!();
                pixel21_0!();
                pixel30_20!();
                pixel31_11!();
            }
            pixel22_31!();
            pixel23_81!();
            pixel32_31!();
            pixel33_81!();
        }
        174 | 46 => {
            if color_diff(w[4], w[2]) {
                pixel00_80!();
                pixel01_10!();
                pixel10_10!();
                pixel11_30!();
            } else {
                pixel00_20!();
                pixel01_12!();
                pixel10_11!();
                pixel11_0!();
            }
            pixel02_32!();
            pixel03_82!();
            pixel12_32!();
            pixel13_82!();
            pixel20_31!();
            pixel21_31!();
            pixel22_70!();
            pixel23_60!();
            pixel30_81!();
            pixel31_81!();
            pixel32_60!();
            pixel33_20!();
        }
        179 | 147 => {
            pixel00_81!();
            pixel01_31!();
            if color_diff(w[2], w[6]) {
                pixel02_10!();
                pixel03_80!();
                pixel12_30!();
                pixel13_10!();
            } else {
                pixel02_11!();
                pixel03_20!();
                pixel12_0!();
                pixel13_12!();
            }
            pixel10_81!();
            pixel11_31!();
            pixel20_60!();
            pixel21_70!();
            pixel22_32!();
            pixel23_32!();
            pixel30_20!();
            pixel31_60!();
            pixel32_82!();
            pixel33_82!();
        }
        117 | 116 => {
            pixel00_20!();
            pixel01_60!();
            pixel02_81!();
            pixel03_81!();
            pixel10_60!();
            pixel11_70!();
            pixel12_31!();
            pixel13_31!();
            pixel20_82!();
            pixel21_32!();
            if color_diff(w[6], w[8]) {
                pixel22_30!();
                pixel23_10!();
                pixel32_10!();
                pixel33_80!();
            } else {
                pixel22_0!();
                pixel23_11!();
                pixel32_12!();
                pixel33_20!();
            }
            pixel30_82!();
            pixel31_32!();
        }
        189 => {
            pixel00_82!();
            pixel01_82!();
            pixel02_81!();
            pixel03_81!();
            pixel10_32!();
            pixel11_32!();
            pixel12_31!();
            pixel13_31!();
            pixel20_31!();
            pixel21_31!();
            pixel22_32!();
            pixel23_32!();
            pixel30_81!();
            pixel31_81!();
            pixel32_82!();
            pixel33_82!();
        }
        231 => {
            pixel00_81!();
            pixel01_31!();
            pixel02_32!();
            pixel03_82!();
            pixel10_81!();
            pixel11_31!();
            pixel12_32!();
            pixel13_82!();
            pixel20_82!();
            pixel21_32!();
            pixel22_31!();
            pixel23_81!();
            pixel30_82!();
            pixel31_32!();
            pixel32_31!();
            pixel33_81!();
        }
        126 => {
            pixel00_80!();
            pixel01_10!();
            if color_diff(w[2], w[6]) {
                pixel02_0!();
                pixel03_0!();
                pixel13_0!();
            } else {
                pixel02_50!();
                pixel03_50!();
                pixel13_50!();
            }
            pixel10_10!();
            pixel11_30!();
            pixel12_0!();
            if color_diff(w[8], w[4]) {
                pixel20_0!();
                pixel30_0!();
                pixel31_0!();
            } else {
                pixel20_50!();
                pixel30_50!();
                pixel31_50!();
            }
            pixel21_0!();
            pixel22_30!();
            pixel23_10!();
            pixel32_10!();
            pixel33_80!();
        }
        219 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
                pixel01_0!();
                pixel10_0!();
            } else {
                pixel00_50!();
                pixel01_50!();
                pixel10_50!();
            }
            pixel02_10!();
            pixel03_80!();
            pixel11_0!();
            pixel12_30!();
            pixel13_10!();
            pixel20_10!();
            pixel21_30!();
            pixel22_0!();
            if color_diff(w[6], w[8]) {
                pixel23_0!();
                pixel32_0!();
                pixel33_0!();
            } else {
                pixel23_50!();
                pixel32_50!();
                pixel33_50!();
            }
            pixel30_80!();
            pixel31_10!();
        }
        125 => {
            if color_diff(w[8], w[4]) {
                pixel00_82!();
                pixel10_32!();
                pixel20_0!();
                pixel21_0!();
                pixel30_0!();
                pixel31_0!();
            } else {
                pixel00_11!();
                pixel10_13!();
                pixel20_83!();
                pixel21_70!();
                pixel30_50!();
                pixel31_21!();
            }
            pixel01_82!();
            pixel02_81!();
            pixel03_81!();
            pixel11_32!();
            pixel12_31!();
            pixel13_31!();
            pixel22_30!();
            pixel23_10!();
            pixel32_10!();
            pixel33_80!();
        }
        221 => {
            pixel00_82!();
            pixel01_82!();
            pixel02_81!();
            if color_diff(w[6], w[8]) {
                pixel03_81!();
                pixel13_31!();
                pixel22_0!();
                pixel23_0!();
                pixel32_0!();
                pixel33_0!();
            } else {
                pixel03_12!();
                pixel13_14!();
                pixel22_70!();
                pixel23_83!();
                pixel32_21!();
                pixel33_50!();
            }
            pixel10_32!();
            pixel11_32!();
            pixel12_31!();
            pixel20_10!();
            pixel21_30!();
            pixel30_80!();
            pixel31_10!();
        }
        207 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
                pixel01_0!();
                pixel02_32!();
                pixel03_82!();
                pixel10_0!();
                pixel11_0!();
            } else {
                pixel00_50!();
                pixel01_83!();
                pixel02_13!();
                pixel03_11!();
                pixel10_21!();
                pixel11_70!();
            }
            pixel12_32!();
            pixel13_82!();
            pixel20_10!();
            pixel21_30!();
            pixel22_31!();
            pixel23_81!();
            pixel30_80!();
            pixel31_10!();
            pixel32_31!();
            pixel33_81!();
        }
        238 => {
            pixel00_80!();
            pixel01_10!();
            pixel02_32!();
            pixel03_82!();
            pixel10_10!();
            pixel11_30!();
            pixel12_32!();
            pixel13_82!();
            if color_diff(w[8], w[4]) {
                pixel20_0!();
                pixel21_0!();
                pixel30_0!();
                pixel31_0!();
                pixel32_31!();
                pixel33_81!();
            } else {
                pixel20_21!();
                pixel21_70!();
                pixel30_50!();
                pixel31_83!();
                pixel32_14!();
                pixel33_12!();
            }
            pixel22_31!();
            pixel23_81!();
        }
        190 => {
            pixel00_80!();
            pixel01_10!();
            if color_diff(w[2], w[6]) {
                pixel02_0!();
                pixel03_0!();
                pixel12_0!();
                pixel13_0!();
                pixel23_32!();
                pixel33_82!();
            } else {
                pixel02_21!();
                pixel03_50!();
                pixel12_70!();
                pixel13_83!();
                pixel23_13!();
                pixel33_11!();
            }
            pixel10_10!();
            pixel11_30!();
            pixel20_31!();
            pixel21_31!();
            pixel22_32!();
            pixel30_81!();
            pixel31_81!();
            pixel32_82!();
        }
        187 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
                pixel01_0!();
                pixel10_0!();
                pixel11_0!();
                pixel20_31!();
                pixel30_81!();
            } else {
                pixel00_50!();
                pixel01_21!();
                pixel10_83!();
                pixel11_70!();
                pixel20_14!();
                pixel30_12!();
            }
            pixel02_10!();
            pixel03_80!();
            pixel12_30!();
            pixel13_10!();
            pixel21_31!();
            pixel22_32!();
            pixel23_32!();
            pixel31_81!();
            pixel32_82!();
            pixel33_82!();
        }
        243 => {
            pixel00_81!();
            pixel01_31!();
            pixel02_10!();
            pixel03_80!();
            pixel10_81!();
            pixel11_31!();
            pixel12_30!();
            pixel13_10!();
            pixel20_82!();
            pixel21_32!();
            if color_diff(w[6], w[8]) {
                pixel22_0!();
                pixel23_0!();
                pixel30_82!();
                pixel31_32!();
                pixel32_0!();
                pixel33_0!();
            } else {
                pixel22_70!();
                pixel23_21!();
                pixel30_11!();
                pixel31_13!();
                pixel32_83!();
                pixel33_50!();
            }
        }
        119 => {
            if color_diff(w[2], w[6]) {
                pixel00_81!();
                pixel01_31!();
                pixel02_0!();
                pixel03_0!();
                pixel12_0!();
                pixel13_0!();
            } else {
                pixel00_12!();
                pixel01_14!();
                pixel02_83!();
                pixel03_50!();
                pixel12_70!();
                pixel13_21!();
            }
            pixel10_81!();
            pixel11_31!();
            pixel20_82!();
            pixel21_32!();
            pixel22_30!();
            pixel23_10!();
            pixel30_82!();
            pixel31_32!();
            pixel32_10!();
            pixel33_80!();
        }
        237 | 233 => {
            pixel00_82!();
            pixel01_82!();
            pixel02_60!();
            pixel03_20!();
            pixel10_32!();
            pixel11_32!();
            pixel12_70!();
            pixel13_60!();
            pixel20_0!();
            pixel21_0!();
            pixel22_31!();
            pixel23_81!();
            if color_diff(w[8], w[4]) {
                pixel30_0!();
            } else {
                pixel30_20!();
            }
            pixel31_0!();
            pixel32_31!();
            pixel33_81!();
        }
        175 | 47 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
            } else {
                pixel00_20!();
            }
            pixel01_0!();
            pixel02_32!();
            pixel03_82!();
            pixel10_0!();
            pixel11_0!();
            pixel12_32!();
            pixel13_82!();
            pixel20_31!();
            pixel21_31!();
            pixel22_70!();
            pixel23_60!();
            pixel30_81!();
            pixel31_81!();
            pixel32_60!();
            pixel33_20!();
        }
        183 | 151 => {
            pixel00_81!();
            pixel01_31!();
            pixel02_0!();
            if color_diff(w[2], w[6]) {
                pixel03_0!();
            } else {
                pixel03_20!();
            }
            pixel10_81!();
            pixel11_31!();
            pixel12_0!();
            pixel13_0!();
            pixel20_60!();
            pixel21_70!();
            pixel22_32!();
            pixel23_32!();
            pixel30_20!();
            pixel31_60!();
            pixel32_82!();
            pixel33_82!();
        }
        245 | 244 => {
            pixel00_20!();
            pixel01_60!();
            pixel02_81!();
            pixel03_81!();
            pixel10_60!();
            pixel11_70!();
            pixel12_31!();
            pixel13_31!();
            pixel20_82!();
            pixel21_32!();
            pixel22_0!();
            pixel23_0!();
            pixel30_82!();
            pixel31_32!();
            pixel32_0!();
            if color_diff(w[6], w[8]) {
                pixel33_0!();
            } else {
                pixel33_20!();
            }
        }
        250 => {
            pixel00_80!();
            pixel01_10!();
            pixel02_10!();
            pixel03_80!();
            pixel10_10!();
            pixel11_30!();
            pixel12_30!();
            pixel13_10!();
            if color_diff(w[8], w[4]) {
                pixel20_0!();
                pixel30_0!();
                pixel31_0!();
            } else {
                pixel20_50!();
                pixel30_50!();
                pixel31_50!();
            }
            pixel21_0!();
            pixel22_0!();
            if color_diff(w[6], w[8]) {
                pixel23_0!();
                pixel32_0!();
                pixel33_0!();
            } else {
                pixel23_50!();
                pixel32_50!();
                pixel33_50!();
            }
        }
        123 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
                pixel01_0!();
                pixel10_0!();
            } else {
                pixel00_50!();
                pixel01_50!();
                pixel10_50!();
            }
            pixel02_10!();
            pixel03_80!();
            pixel11_0!();
            pixel12_30!();
            pixel13_10!();
            if color_diff(w[8], w[4]) {
                pixel20_0!();
                pixel30_0!();
                pixel31_0!();
            } else {
                pixel20_50!();
                pixel30_50!();
                pixel31_50!();
            }
            pixel21_0!();
            pixel22_30!();
            pixel23_10!();
            pixel32_10!();
            pixel33_80!();
        }
        95 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
                pixel01_0!();
                pixel10_0!();
            } else {
                pixel00_50!();
                pixel01_50!();
                pixel10_50!();
            }
            if color_diff(w[2], w[6]) {
                pixel02_0!();
                pixel03_0!();
                pixel13_0!();
            } else {
                pixel02_50!();
                pixel03_50!();
                pixel13_50!();
            }
            pixel11_0!();
            pixel12_0!();
            pixel20_10!();
            pixel21_30!();
            pixel22_30!();
            pixel23_10!();
            pixel30_80!();
            pixel31_10!();
            pixel32_10!();
            pixel33_80!();
        }
        222 => {
            pixel00_80!();
            pixel01_10!();
            if color_diff(w[2], w[6]) {
                pixel02_0!();
                pixel03_0!();
                pixel13_0!();
            } else {
                pixel02_50!();
                pixel03_50!();
                pixel13_50!();
            }
            pixel10_10!();
            pixel11_30!();
            pixel12_0!();
            pixel20_10!();
            pixel21_30!();
            pixel22_0!();
            if color_diff(w[6], w[8]) {
                pixel23_0!();
                pixel32_0!();
                pixel33_0!();
            } else {
                pixel23_50!();
                pixel32_50!();
                pixel33_50!();
            }
            pixel30_80!();
            pixel31_10!();
        }
        252 => {
            pixel00_80!();
            pixel01_61!();
            pixel02_81!();
            pixel03_81!();
            pixel10_10!();
            pixel11_30!();
            pixel12_31!();
            pixel13_31!();
            if color_diff(w[8], w[4]) {
                pixel20_0!();
                pixel30_0!();
                pixel31_0!();
            } else {
                pixel20_50!();
                pixel30_50!();
                pixel31_50!();
            }
            pixel21_0!();
            pixel22_0!();
            pixel23_0!();
            pixel32_0!();
            if color_diff(w[6], w[8]) {
                pixel33_0!();
            } else {
                pixel33_20!();
            }
        }
        249 => {
            pixel00_82!();
            pixel01_82!();
            pixel02_61!();
            pixel03_80!();
            pixel10_32!();
            pixel11_32!();
            pixel12_30!();
            pixel13_10!();
            pixel20_0!();
            pixel21_0!();
            pixel22_0!();
            if color_diff(w[6], w[8]) {
                pixel23_0!();
                pixel32_0!();
                pixel33_0!();
            } else {
                pixel23_50!();
                pixel32_50!();
                pixel33_50!();
            }
            if color_diff(w[8], w[4]) {
                pixel30_0!();
            } else {
                pixel30_20!();
            }
            pixel31_0!();
        }
        235 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
                pixel01_0!();
                pixel10_0!();
            } else {
                pixel00_50!();
                pixel01_50!();
                pixel10_50!();
            }
            pixel02_10!();
            pixel03_80!();
            pixel11_0!();
            pixel12_30!();
            pixel13_61!();
            pixel20_0!();
            pixel21_0!();
            pixel22_31!();
            pixel23_81!();
            if color_diff(w[8], w[4]) {
                pixel30_0!();
            } else {
                pixel30_20!();
            }
            pixel31_0!();
            pixel32_31!();
            pixel33_81!();
        }
        111 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
            } else {
                pixel00_20!();
            }
            pixel01_0!();
            pixel02_32!();
            pixel03_82!();
            pixel10_0!();
            pixel11_0!();
            pixel12_32!();
            pixel13_82!();
            if color_diff(w[8], w[4]) {
                pixel20_0!();
                pixel30_0!();
                pixel31_0!();
            } else {
                pixel20_50!();
                pixel30_50!();
                pixel31_50!();
            }
            pixel21_0!();
            pixel22_30!();
            pixel23_61!();
            pixel32_10!();
            pixel33_80!();
        }
        63 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
            } else {
                pixel00_20!();
            }
            pixel01_0!();
            if color_diff(w[2], w[6]) {
                pixel02_0!();
                pixel03_0!();
                pixel13_0!();
            } else {
                pixel02_50!();
                pixel03_50!();
                pixel13_50!();
            }
            pixel10_0!();
            pixel11_0!();
            pixel12_0!();
            pixel20_31!();
            pixel21_31!();
            pixel22_30!();
            pixel23_10!();
            pixel30_81!();
            pixel31_81!();
            pixel32_61!();
            pixel33_80!();
        }
        159 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
                pixel01_0!();
                pixel10_0!();
            } else {
                pixel00_50!();
                pixel01_50!();
                pixel10_50!();
            }
            pixel02_0!();
            if color_diff(w[2], w[6]) {
                pixel03_0!();
            } else {
                pixel03_20!();
            }
            pixel11_0!();
            pixel12_0!();
            pixel13_0!();
            pixel20_10!();
            pixel21_30!();
            pixel22_32!();
            pixel23_32!();
            pixel30_80!();
            pixel31_61!();
            pixel32_82!();
            pixel33_82!();
        }
        215 => {
            pixel00_81!();
            pixel01_31!();
            pixel02_0!();
            if color_diff(w[2], w[6]) {
                pixel03_0!();
            } else {
                pixel03_20!();
            }
            pixel10_81!();
            pixel11_31!();
            pixel12_0!();
            pixel13_0!();
            pixel20_61!();
            pixel21_30!();
            pixel22_0!();
            if color_diff(w[6], w[8]) {
                pixel23_0!();
                pixel32_0!();
                pixel33_0!();
            } else {
                pixel23_50!();
                pixel32_50!();
                pixel33_50!();
            }
            pixel30_80!();
            pixel31_10!();
        }
        246 => {
            pixel00_80!();
            pixel01_10!();
            if color_diff(w[2], w[6]) {
                pixel02_0!();
                pixel03_0!();
                pixel13_0!();
            } else {
                pixel02_50!();
                pixel03_50!();
                pixel13_50!();
            }
            pixel10_61!();
            pixel11_30!();
            pixel12_0!();
            pixel20_82!();
            pixel21_32!();
            pixel22_0!();
            pixel23_0!();
            pixel30_82!();
            pixel31_32!();
            pixel32_0!();
            if color_diff(w[6], w[8]) {
                pixel33_0!();
            } else {
                pixel33_20!();
            }
        }
        254 => {
            pixel00_80!();
            pixel01_10!();
            if color_diff(w[2], w[6]) {
                pixel02_0!();
                pixel03_0!();
                pixel13_0!();
            } else {
                pixel02_50!();
                pixel03_50!();
                pixel13_50!();
            }
            pixel10_10!();
            pixel11_30!();
            pixel12_0!();
            if color_diff(w[8], w[4]) {
                pixel20_0!();
                pixel30_0!();
                pixel31_0!();
            } else {
                pixel20_50!();
                pixel30_50!();
                pixel31_50!();
            }
            pixel21_0!();
            pixel22_0!();
            pixel23_0!();
            pixel32_0!();
            if color_diff(w[6], w[8]) {
                pixel33_0!();
            } else {
                pixel33_20!();
            }
        }
        253 => {
            pixel00_82!();
            pixel01_82!();
            pixel02_81!();
            pixel03_81!();
            pixel10_32!();
            pixel11_32!();
            pixel12_31!();
            pixel13_31!();
            pixel20_0!();
            pixel21_0!();
            pixel22_0!();
            pixel23_0!();
            if color_diff(w[8], w[4]) {
                pixel30_0!();
            } else {
                pixel30_20!();
            }
            pixel31_0!();
            pixel32_0!();
            if color_diff(w[6], w[8]) {
                pixel33_0!();
            } else {
                pixel33_20!();
            }
        }
        251 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
                pixel01_0!();
                pixel10_0!();
            } else {
                pixel00_50!();
                pixel01_50!();
                pixel10_50!();
            }
            pixel02_10!();
            pixel03_80!();
            pixel11_0!();
            pixel12_30!();
            pixel13_10!();
            pixel20_0!();
            pixel21_0!();
            pixel22_0!();
            if color_diff(w[6], w[8]) {
                pixel23_0!();
                pixel32_0!();
                pixel33_0!();
            } else {
                pixel23_50!();
                pixel32_50!();
                pixel33_50!();
            }
            if color_diff(w[8], w[4]) {
                pixel30_0!();
            } else {
                pixel30_20!();
            }
            pixel31_0!();
        }
        239 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
            } else {
                pixel00_20!();
            }
            pixel01_0!();
            pixel02_32!();
            pixel03_82!();
            pixel10_0!();
            pixel11_0!();
            pixel12_32!();
            pixel13_82!();
            pixel20_0!();
            pixel21_0!();
            pixel22_31!();
            pixel23_81!();
            if color_diff(w[8], w[4]) {
                pixel30_0!();
            } else {
                pixel30_20!();
            }
            pixel31_0!();
            pixel32_31!();
            pixel33_81!();
        }
        127 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
            } else {
                pixel00_20!();
            }
            pixel01_0!();
            if color_diff(w[2], w[6]) {
                pixel02_0!();
                pixel03_0!();
                pixel13_0!();
            } else {
                pixel02_50!();
                pixel03_50!();
                pixel13_50!();
            }
            pixel10_0!();
            pixel11_0!();
            pixel12_0!();
            if color_diff(w[8], w[4]) {
                pixel20_0!();
                pixel30_0!();
                pixel31_0!();
            } else {
                pixel20_50!();
                pixel30_50!();
                pixel31_50!();
            }
            pixel21_0!();
            pixel22_30!();
            pixel23_10!();
            pixel32_10!();
            pixel33_80!();
        }
        191 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
            } else {
                pixel00_20!();
            }
            pixel01_0!();
            pixel02_0!();
            if color_diff(w[2], w[6]) {
                pixel03_0!();
            } else {
                pixel03_20!();
            }
            pixel10_0!();
            pixel11_0!();
            pixel12_0!();
            pixel13_0!();
            pixel20_31!();
            pixel21_31!();
            pixel22_32!();
            pixel23_32!();
            pixel30_81!();
            pixel31_81!();
            pixel32_82!();
            pixel33_82!();
        }
        223 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
                pixel01_0!();
                pixel10_0!();
            } else {
                pixel00_50!();
                pixel01_50!();
                pixel10_50!();
            }
            pixel02_0!();
            if color_diff(w[2], w[6]) {
                pixel03_0!();
            } else {
                pixel03_20!();
            }
            pixel11_0!();
            pixel12_0!();
            pixel13_0!();
            pixel20_10!();
            pixel21_30!();
            pixel22_0!();
            if color_diff(w[6], w[8]) {
                pixel23_0!();
                pixel32_0!();
                pixel33_0!();
            } else {
                pixel23_50!();
                pixel32_50!();
                pixel33_50!();
            }
            pixel30_80!();
            pixel31_10!();
        }
        247 => {
            pixel00_81!();
            pixel01_31!();
            pixel02_0!();
            if color_diff(w[2], w[6]) {
                pixel03_0!();
            } else {
                pixel03_20!();
            }
            pixel10_81!();
            pixel11_31!();
            pixel12_0!();
            pixel13_0!();
            pixel20_82!();
            pixel21_32!();
            pixel22_0!();
            pixel23_0!();
            pixel30_82!();
            pixel31_32!();
            pixel32_0!();
            if color_diff(w[6], w[8]) {
                pixel33_0!();
            } else {
                pixel33_20!();
            }
        }
        255 => {
            if color_diff(w[4], w[2]) {
                pixel00_0!();
            } else {
                pixel00_20!();
            }
            pixel01_0!();
            pixel02_0!();
            if color_diff(w[2], w[6]) {
                pixel03_0!();
            } else {
                pixel03_20!();
            }
            pixel10_0!();
            pixel11_0!();
            pixel12_0!();
            pixel13_0!();
            pixel20_0!();
            pixel21_0!();
            pixel22_0!();
            pixel23_0!();
            if color_diff(w[8], w[4]) {
                pixel30_0!();
            } else {
                pixel30_20!();
            }
            pixel31_0!();
            pixel32_0!();
            if color_diff(w[6], w[8]) {
                pixel33_0!();
            } else {
                pixel33_20!();
            }
        }
        _ => unreachable!(),
    }
}
