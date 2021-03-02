use crate::bus::*;
use crate::system::nes::Cartridge;
use crate::types::*;
use crate::video::*;
use std::num::Wrapping;

pub type Address = u14w;
pub type Word = u8w;

const ADDR_CONTROL: cpu::cpu6502::Address = Wrapping(0);
const ADDR_MASK: cpu::cpu6502::Address = Wrapping(1);
const ADDR_STATUS: cpu::cpu6502::Address = Wrapping(2);
const ADDR_OAM_ADDRESS: cpu::cpu6502::Address = Wrapping(3);
const ADDR_OAM_DATA: cpu::cpu6502::Address = Wrapping(4);
const ADDR_SCROLL: cpu::cpu6502::Address = Wrapping(5);
const ADDR_PPU_ADDRESS: cpu::cpu6502::Address = Wrapping(6);
const ADDR_PPU_DATA: cpu::cpu6502::Address = Wrapping(7);
const ADDR_MAX: cpu::cpu6502::Address = ADDR_PPU_DATA;

pub const SCREEN_WIDTH: usize = 256;
pub const SCREEN_HEIGHT: usize = 240;

const MAX_CYCLE: u16 = 340;
const MAX_SCANLINE: i16 = 260;
const HBLANK_CYCLE: u16 = 256;
const VBLANK_LINE: i16 = 240;

// Helper function to keep some code below clean
#[inline]
fn select<T>(eval: bool, if_true: T, if_false: T) -> T {
    if eval {
        if_true
    } else {
        if_false
    }
}

fn flip_byte(mut b: u8) -> u8 {
    b = ((b & 0xF0) >> 4) | ((b & 0x0F) << 4);
    b = ((b & 0xCC) >> 2) | ((b & 0x33) << 2);
    b = ((b & 0xAA) >> 1) | ((b & 0x55) << 1);
    b
}

const NES_PALETTE: [Color; 64] = [
    Color::from_rgb(84, 84, 84),
    Color::from_rgb(0, 30, 116),
    Color::from_rgb(8, 16, 144),
    Color::from_rgb(48, 0, 136),
    Color::from_rgb(68, 0, 100),
    Color::from_rgb(92, 0, 48),
    Color::from_rgb(84, 4, 0),
    Color::from_rgb(60, 24, 0),
    Color::from_rgb(32, 42, 0),
    Color::from_rgb(8, 58, 0),
    Color::from_rgb(0, 64, 0),
    Color::from_rgb(0, 60, 0),
    Color::from_rgb(0, 50, 60),
    Color::BLACK,
    Color::BLACK,
    Color::BLACK,
    Color::from_rgb(152, 150, 152),
    Color::from_rgb(8, 76, 196),
    Color::from_rgb(48, 50, 236),
    Color::from_rgb(92, 30, 228),
    Color::from_rgb(136, 20, 176),
    Color::from_rgb(160, 20, 100),
    Color::from_rgb(152, 34, 32),
    Color::from_rgb(120, 60, 0),
    Color::from_rgb(84, 90, 0),
    Color::from_rgb(40, 114, 0),
    Color::from_rgb(8, 124, 0),
    Color::from_rgb(0, 118, 40),
    Color::from_rgb(0, 102, 120),
    Color::BLACK,
    Color::BLACK,
    Color::BLACK,
    Color::from_rgb(236, 238, 236),
    Color::from_rgb(76, 154, 236),
    Color::from_rgb(120, 124, 236),
    Color::from_rgb(176, 98, 236),
    Color::from_rgb(228, 84, 236),
    Color::from_rgb(236, 88, 180),
    Color::from_rgb(236, 106, 100),
    Color::from_rgb(212, 136, 32),
    Color::from_rgb(160, 170, 0),
    Color::from_rgb(116, 196, 0),
    Color::from_rgb(76, 208, 32),
    Color::from_rgb(56, 204, 108),
    Color::from_rgb(56, 180, 204),
    Color::from_rgb(60, 60, 60),
    Color::BLACK,
    Color::BLACK,
    Color::from_rgb(236, 238, 236),
    Color::from_rgb(168, 204, 236),
    Color::from_rgb(188, 188, 236),
    Color::from_rgb(212, 178, 236),
    Color::from_rgb(236, 174, 236),
    Color::from_rgb(236, 174, 212),
    Color::from_rgb(236, 180, 176),
    Color::from_rgb(228, 196, 144),
    Color::from_rgb(204, 210, 120),
    Color::from_rgb(180, 222, 120),
    Color::from_rgb(168, 226, 144),
    Color::from_rgb(152, 226, 180),
    Color::from_rgb(160, 214, 228),
    Color::from_rgb(160, 162, 160),
    Color::BLACK,
    Color::BLACK,
];

pub struct PixelBuffer {
    pixels: [Color; SCREEN_WIDTH * SCREEN_HEIGHT],
}
impl PixelBuffer {
    #[inline]
    pub const fn new() -> Self {
        Self {
            pixels: [Color::WHITE; SCREEN_WIDTH * SCREEN_HEIGHT],
        }
    }
}
impl PixelBuffer {
    #[inline]
    fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        let index = (y * SCREEN_WIDTH) + x;
        self.pixels[index] = color;
    }
}
impl VideoBuffer for PixelBuffer {
    #[inline]
    fn width(&self) -> usize {
        SCREEN_WIDTH
    }
    #[inline]
    fn height(&self) -> usize {
        SCREEN_HEIGHT
    }

    #[inline]
    fn get_pixels(&self) -> &[Color] {
        &self.pixels
    }
}

bitflags! {
    struct PpuControl : u8 {
        const NAMETABLE_X        = 0b00000001;
        const NAMETABLE_Y        = 0b00000010;
        const INCREMENT_MODE     = 0b00000100;
        const PATTERN_SPRITE     = 0b00001000;
        const PATTERN_BACKGROUND = 0b00010000;
        const SPRITE_SIZE        = 0b00100000;
        const SLAVE_MODE         = 0b01000000;
        const ENABLE_NMI         = 0b10000000;
    }
}

bitflags! {
    struct PpuMask : u8 {
        const GREYSCALE              = 0b00000001;
        const RENDER_BACKGROUND_LEFT = 0b00000010;
        const RENDER_SPRITES_LEFT    = 0b00000100;
        const RENDER_BACKGROUND      = 0b00001000;
        const RENDER_SPRITES         = 0b00010000;
        const ENHANCE_RED            = 0b00100000;
        const ENHANCE_GREEN          = 0b01000000;
        const ENHANCE_BLUE           = 0b10000000;
    }
}

bitflags! {
    struct PpuStatus : u8 {
        const SPRITE_OVERFLOW = 0b00100000;
        const SPRITE_ZERO_HIT = 0b01000000;
        const VERTICAL_BLANK  = 0b10000000;
    }
}

bitflags! {
    struct SpriteAttributes : u8 {
        const FLIP_VERT = 0x80;
        const FLIP_HOR = 0x40;
        const PRIORITY = 0x20;
    }
}

#[derive(Clone, Copy, Debug)]
struct ObjectAttributes {
    attribs: [Wrapping<u8>; 4],
}
impl ObjectAttributes {
    #[inline]
    const fn new() -> Self {
        Self {
            attribs: [Wrapping(0xFF); 4],
        }
    }

    #[inline]
    const fn x(&self) -> u8 {
        self.attribs[3].0
    }
    #[inline]
    const fn y(&self) -> u8 {
        self.attribs[0].0
    }
    #[inline]
    const fn id(&self) -> u8 {
        self.attribs[1].0
    }
    #[inline]
    const fn attr(&self) -> SpriteAttributes {
        SpriteAttributes::from_bits_truncate(self.attribs[2].0)
    }
    #[inline]
    const fn palette(&self) -> u8 {
        (self.attribs[2].0 & 0x03) + 0x04
    }

    #[inline]
    fn dec_x(&mut self) {
        self.attribs[3] -= Wrapping(1);
    }
}

struct ObjectAttributeMemory {
    entries: [ObjectAttributes; 64],
}
impl ObjectAttributeMemory {
    #[inline]
    const fn new() -> Self {
        Self {
            entries: [ObjectAttributes::new(); 64],
        }
    }

    #[inline]
    const fn get(&self, index: usize) -> ObjectAttributes {
        self.entries[index]
    }

    const fn read(&self, addr: Wrapping<u8>) -> Wrapping<u8> {
        let index = (addr.0 as usize) / 4;
        let offset = (addr.0 as usize) - (index * 4);
        self.entries[index].attribs[offset]
    }

    fn write(&mut self, addr: Wrapping<u8>, data: Wrapping<u8>) {
        let index = (addr.0 as usize) / 4;
        let offset = (addr.0 as usize) - (index * 4);
        self.entries[index].attribs[offset] = data;
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
struct PpuRegister {
    value: u16,
    coarse_x: u16,
    coarse_y: u16,
    nametable_x: u16,
    nametable_y: u16,
    fine_y: u16,
}
impl PpuRegister {
    #[inline]
    const fn new() -> Self {
        Self {
            value: 0,
            coarse_x: 0,
            coarse_y: 0,
            nametable_x: 0,
            nametable_y: 0,
            fine_y: 0,
        }
    }

    fn update_subfields(&mut self) {
        self.coarse_x = self.value & 0x001F;
        self.coarse_y = (self.value >> 5) & 0x001F;
        self.nametable_x = (self.value >> 10) & 0x0001;
        self.nametable_y = (self.value >> 11) & 0x0001;
        self.fine_y = (self.value >> 12) & 0x0007;
    }

    fn update_value(&mut self) {
        self.value = (self.coarse_x & 0x001F)
            | ((self.coarse_y & 0x001F) << 5)
            | ((self.nametable_x & 0x0001) << 10)
            | ((self.nametable_y & 0x0001) << 11)
            | ((self.fine_y & 0x0007) << 12);
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
struct PpuShiftRegister {
    value: u16,
}
impl PpuShiftRegister {
    #[inline]
    const fn new() -> Self {
        Self { value: 0 }
    }

    #[inline]
    fn shift(&mut self) {
        self.value <<= 1;
    }
}

pub struct Ppu2C02<'a> {
    bus: EmuRef<Bus<'a, Address, Word>>,
    range: AddressRange<cpu::cpu6502::Address>,
    cartridge: Option<EmuRef<Cartridge>>,

    oam: ObjectAttributeMemory,
    scanline: i16,
    cycle: u16,
    back_buffer: Box<PixelBuffer>,
    front_buffer: Box<PixelBuffer>,
    control: PpuControl,
    mask: PpuMask,
    status: PpuStatus,
    ppu_addr_latch: bool,
    ppu_data_buffer: Wrapping<u8>,
    nmi: bool,
    vram_addr: PpuRegister,
    tram_addr: PpuRegister,
    fine_x: u8,
    bg_next_id: u8,
    bg_next_attr: u8,
    bg_next_lsb: u8,
    bg_next_msb: u8,
    bg_pattern_lo: PpuShiftRegister,
    bg_pattern_hi: PpuShiftRegister,
    bg_attr_lo: PpuShiftRegister,
    bg_attr_hi: PpuShiftRegister,
    oam_addr: Wrapping<u8>,
    sprites_line: [ObjectAttributes; 8],
    sprite_count: usize,
    sprite_pattern_lo: [u8; 8],
    sprite_pattern_hi: [u8; 8],
    allow_zero_hit: bool,
}
impl<'a> Ppu2C02<'a> {
    pub fn new(bus: EmuRef<Bus<'a, Address, Word>>, range_start: cpu::cpu6502::Address) -> Self {
        let oam = ObjectAttributeMemory::new();

        Self {
            bus,
            range: AddressRange::new(range_start, range_start + ADDR_MAX),
            cartridge: None,
            oam,
            scanline: 0,
            cycle: 0,
            back_buffer: Box::new(PixelBuffer::new()),
            front_buffer: Box::new(PixelBuffer::new()),
            control: PpuControl::empty(),
            mask: PpuMask::empty(),
            status: PpuStatus::empty(),
            ppu_addr_latch: false,
            ppu_data_buffer: Wrapping(0),
            nmi: false,
            vram_addr: PpuRegister::new(),
            tram_addr: PpuRegister::new(),
            fine_x: 0,
            bg_next_id: 0,
            bg_next_attr: 0,
            bg_next_lsb: 0,
            bg_next_msb: 0,
            bg_pattern_lo: PpuShiftRegister::new(),
            bg_pattern_hi: PpuShiftRegister::new(),
            bg_attr_lo: PpuShiftRegister::new(),
            bg_attr_hi: PpuShiftRegister::new(),
            oam_addr: Wrapping(0),
            sprites_line: [ObjectAttributes::new(); 8],
            sprite_count: 0,
            sprite_pattern_lo: [0; 8],
            sprite_pattern_hi: [0; 8],
            allow_zero_hit: false,
        }
    }

    pub fn create(
        bus: EmuRef<Bus<'a, Address, Word>>,
        range_start: cpu::cpu6502::Address,
    ) -> EmuRef<Self> {
        make_ref(Self::new(bus, range_start))
    }

    pub fn check_nmi(&mut self) -> bool {
        let tmp = self.nmi;
        self.nmi = false;
        tmp
    }

    #[inline]
    pub fn set_cartridge(&mut self, cartridge: EmuRef<Cartridge>) {
        self.cartridge = Some(cartridge);
    }

    #[inline]
    pub fn remove_cartridge(&mut self) {
        self.cartridge = None;
    }

    fn read_bus(&self, mut addr: Address) -> Word {
        if addr >= 0x3F00 {
            addr &= 0x001F;
            if (addr & 0x000F) % 4 == 0 {
                addr = Address::ZERO;
            }
            addr |= 0x3F00;
        }
        let bus_borrow = self.bus.borrow();
        bus_borrow.read(addr)
    }

    fn write_bus(&self, mut addr: Address, data: Word) {
        if addr >= 0x3F00 {
            addr &= 0x001F;
            if (addr & 0x000F) % 4 == 0 {
                addr &= 0x000F;
            }
            addr |= 0x3F00;
        }
        let bus_borrow = self.bus.borrow();
        bus_borrow.write(addr, data);
    }

    fn get_palette_color(&self, palette: Address, pixel: u8w) -> Color {
        // A pixel with value of 0 always mirrors to the first color in the palette (background)
        const BASE_ADDR: Address = Address::new(0x3F00);
        let addr = BASE_ADDR + (palette * Address::new(4)) + Address::new(pixel.0 as u16);
        let color_index =
            self.read_bus(addr).0 & select(self.mask.contains(PpuMask::GREYSCALE), 0x30, 0x3F);
        NES_PALETTE[color_index as usize]
    }

    fn inc_x(&mut self) {
        if self
            .mask
            .intersects(PpuMask::RENDER_BACKGROUND | PpuMask::RENDER_SPRITES)
        {
            if self.vram_addr.coarse_x == 31 {
                self.vram_addr.coarse_x = 0;
                self.vram_addr.nametable_x = !self.vram_addr.nametable_x & 0x0001;
            } else {
                self.vram_addr.coarse_x += 1;
            }
            self.vram_addr.update_value();
        }
    }

    fn inc_y(&mut self) {
        if self
            .mask
            .intersects(PpuMask::RENDER_BACKGROUND | PpuMask::RENDER_SPRITES)
        {
            if self.vram_addr.fine_y < 7 {
                self.vram_addr.fine_y += 1;
            } else {
                self.vram_addr.fine_y = 0;
                if self.vram_addr.coarse_y == 29 {
                    self.vram_addr.coarse_y = 0;
                    self.vram_addr.nametable_y = !self.vram_addr.nametable_y & 0x0001;
                } else if self.vram_addr.coarse_y == 31 {
                    self.vram_addr.coarse_y = 0;
                } else {
                    self.vram_addr.coarse_y += 1;
                }
            }
            self.vram_addr.update_value();
        }
    }

    fn trans_x(&mut self) {
        if self
            .mask
            .intersects(PpuMask::RENDER_BACKGROUND | PpuMask::RENDER_SPRITES)
        {
            self.vram_addr.nametable_x = self.tram_addr.nametable_x;
            self.vram_addr.coarse_x = self.tram_addr.coarse_x;
            self.vram_addr.update_value();
        }
    }

    fn trans_y(&mut self) {
        if self
            .mask
            .intersects(PpuMask::RENDER_BACKGROUND | PpuMask::RENDER_SPRITES)
        {
            self.vram_addr.fine_y = self.tram_addr.fine_y;
            self.vram_addr.nametable_y = self.tram_addr.nametable_y;
            self.vram_addr.coarse_y = self.tram_addr.coarse_y;
            self.vram_addr.update_value();
        }
    }

    fn load_shifters(&mut self) {
        self.bg_pattern_lo.value &= 0xFF00;
        self.bg_pattern_lo.value |= self.bg_next_lsb as u16;
        self.bg_pattern_hi.value &= 0xFF00;
        self.bg_pattern_hi.value |= self.bg_next_msb as u16;
        self.bg_attr_lo.value &= 0xFF00;
        self.bg_attr_lo.value |= select((self.bg_next_attr & 0x01) != 0, 0x00FF, 0x0000);
        self.bg_attr_hi.value &= 0xFF00;
        self.bg_attr_hi.value |= select((self.bg_next_attr & 0x02) != 0, 0x00FF, 0x0000);
    }

    fn update_shifters(&mut self) {
        if self.mask.contains(PpuMask::RENDER_BACKGROUND) {
            self.bg_pattern_lo.shift();
            self.bg_pattern_hi.shift();
            self.bg_attr_lo.shift();
            self.bg_attr_hi.shift();
        }

        if (self.cycle < 258) && self.mask.contains(PpuMask::RENDER_SPRITES) {
            for i in 0..self.sprite_count {
                let sprite = &mut self.sprites_line[i];
                if sprite.x() > 0 {
                    sprite.dec_x();
                } else {
                    self.sprite_pattern_lo[i] <<= 1;
                    self.sprite_pattern_hi[i] <<= 1;
                }
            }
        }
    }

    fn load_background_data(&mut self) {
        match (self.cycle - 1) % 8 {
            0 => {
                self.load_shifters();
                self.bg_next_id = self
                    .read_bus(Address::new(0x2000 | (self.vram_addr.value & 0x0FFF)))
                    .0;
            }
            2 => {
                self.bg_next_attr = self
                    .read_bus(Address::new(
                        0x23C0
                            | (self.vram_addr.nametable_y << 11)
                            | (self.vram_addr.nametable_x << 10)
                            | ((self.vram_addr.coarse_y >> 2) << 3)
                            | (self.vram_addr.coarse_x >> 2),
                    ))
                    .0;
                if (self.vram_addr.coarse_y & 0x02) != 0 {
                    self.bg_next_attr >>= 4;
                }
                if (self.vram_addr.coarse_x & 0x02) != 0 {
                    self.bg_next_attr >>= 2;
                }
                self.bg_next_attr &= 0x03;
            }
            4 => {
                let bg_table = self.control.contains(PpuControl::PATTERN_BACKGROUND);
                let offset = select(bg_table, 1 << 12, 0);
                let addr = offset + ((self.bg_next_id as u16) << 4) + self.vram_addr.fine_y;
                self.bg_next_lsb = self.read_bus(Address::new(addr)).0;
            }
            6 => {
                let bg_table = self.control.contains(PpuControl::PATTERN_BACKGROUND);
                let offset = select(bg_table, 1 << 12, 0);
                let addr = offset + ((self.bg_next_id as u16) << 4) + self.vram_addr.fine_y + 8;
                self.bg_next_msb = self.read_bus(Address::new(addr)).0;
            }
            7 => self.inc_x(),
            _ => {}
        }
    }

    fn get_sprite_addr(&self, sprite: &ObjectAttributes) -> u16 {
        if self.control.contains(PpuControl::SPRITE_SIZE) {
            // 8x16 mode
            let pattern = ((sprite.id() & 0x01) as u16) << 12;
            let row = ((self.scanline as u16) - (sprite.y() as u16)) & 0x07;
            if sprite.attr().contains(SpriteAttributes::FLIP_VERT) {
                if (self.scanline - (sprite.y() as i16)) < 8 {
                    // Top half
                    pattern | ((((sprite.id() & 0xFE) as u16) + 1) << 4) | (7 - row)
                } else {
                    // Bottom half
                    pattern | (((sprite.id() & 0xFE) as u16) << 4) | (7 - row)
                }
            } else {
                if (self.scanline - (sprite.y() as i16)) < 8 {
                    // Top half
                    pattern | (((sprite.id() & 0xFE) as u16) << 4) | row
                } else {
                    // Bottom half
                    pattern | ((((sprite.id() & 0xFE) as u16) + 1) << 4) | row
                }
            }
        } else {
            // 8x8 mode
            let pattern = select(
                self.control.contains(PpuControl::PATTERN_SPRITE),
                1 << 12,
                0,
            );
            let cell = (sprite.id() as u16) << 4;
            let row = ((self.scanline as u16) - (sprite.y() as u16)) & 0x07;
            if sprite.attr().contains(SpriteAttributes::FLIP_VERT) {
                pattern | cell | (7 - row)
            } else {
                pattern | cell | row
            }
        }
    }

    fn load_foreground_data(&mut self) {
        if (self.cycle == MAX_CYCLE) && (self.scanline >= 0) {
            // Clear sprites
            self.sprites_line = [ObjectAttributes::new(); 8];
            for i in 0..8 {
                self.sprite_pattern_lo[i] = 0;
                self.sprite_pattern_hi[i] = 0;
            }

            let sprite_height = select(self.control.contains(PpuControl::SPRITE_SIZE), 16, 8);

            self.sprite_count = 0;
            let mut oam_index: usize = 0;
            self.allow_zero_hit = false;
            while (oam_index < 64) && (self.sprite_count < 9) {
                let sprite = self.oam.get(oam_index);

                let diff = self.scanline - (sprite.y() as i16);
                if (diff >= 0) && (diff < sprite_height) {
                    if self.sprite_count < 8 {
                        if oam_index == 0 {
                            // Sprite zero hit detection
                            self.allow_zero_hit = true;
                        }

                        self.sprites_line[self.sprite_count] = sprite;
                        self.sprite_count += 1;
                    } else {
                        self.status.insert(PpuStatus::SPRITE_OVERFLOW);
                    }
                }

                oam_index += 1;
            }

            for i in 0..self.sprite_count {
                let sprite = &self.sprites_line[i];
                let addr_lo = self.get_sprite_addr(sprite);
                let addr_hi = addr_lo + 8;

                let mut pattern_lo = self.read_bus(Address::new(addr_lo)).0;
                let mut pattern_hi = self.read_bus(Address::new(addr_hi)).0;
                if sprite.attr().contains(SpriteAttributes::FLIP_HOR) {
                    pattern_lo = flip_byte(pattern_lo);
                    pattern_hi = flip_byte(pattern_hi);
                }

                self.sprite_pattern_lo[i] = pattern_lo;
                self.sprite_pattern_hi[i] = pattern_hi;
            }
        }
    }

    fn clock_one(&mut self) {
        if self.scanline < VBLANK_LINE {
            if (self.scanline == 0) && (self.cycle == 0) {
                self.cycle = 1; // "Odd frame" skip
            }

            if (self.scanline == -1) && (self.cycle == 1) {
                // Start of new frame
                self.status.remove(
                    PpuStatus::VERTICAL_BLANK
                        | PpuStatus::SPRITE_OVERFLOW
                        | PpuStatus::SPRITE_ZERO_HIT,
                );
                for i in 0..8 {
                    self.sprite_pattern_lo[i] = 0;
                    self.sprite_pattern_hi[i] = 0;
                }
            }

            if ((self.cycle > 1) && (self.cycle < 258))
                || ((self.cycle > 320) && (self.cycle < 338))
            {
                self.update_shifters();
                self.load_background_data();
            }

            if self.cycle == HBLANK_CYCLE {
                self.inc_y();
            }
            if self.cycle == (HBLANK_CYCLE + 1) {
                self.load_shifters();
                self.trans_x();
            }
            if (self.scanline == -1) && (self.cycle >= 280) && (self.cycle < 305) {
                self.trans_y();
            }

            self.load_foreground_data();
        }

        if (self.scanline == (VBLANK_LINE + 1)) && (self.cycle == 1) {
            self.status.insert(PpuStatus::VERTICAL_BLANK);
            if self.control.contains(PpuControl::ENABLE_NMI) {
                self.nmi = true;
            }
        }

        let mut bg_pixel: u8 = 0;
        let mut bg_palette: u8 = 0;
        if self.mask.contains(PpuMask::RENDER_BACKGROUND) {
            let mux: u16 = 0x8000 >> self.fine_x;

            let p0: u8 = select((self.bg_pattern_lo.value & mux) != 0, 0x01, 0x00);
            let p1: u8 = select((self.bg_pattern_hi.value & mux) != 0, 0x02, 0x00);
            bg_pixel = p0 | p1;

            let pal0: u8 = select((self.bg_attr_lo.value & mux) != 0, 0x01, 0x00);
            let pal1: u8 = select((self.bg_attr_hi.value & mux) != 0, 0x02, 0x00);
            bg_palette = pal0 | pal1;
        }

        let mut fg_pixel: u8 = 0;
        let mut fg_palette: u8 = 0;
        let mut fg_priority: bool = false;
        let mut zero_visible = false;
        if self.mask.contains(PpuMask::RENDER_SPRITES) {
            for i in 0..self.sprite_count {
                let sprite = &self.sprites_line[i];
                if sprite.x() == 0 {
                    let p0: u8 = (self.sprite_pattern_lo[i] & 0x80) >> 7;
                    let p1: u8 = (self.sprite_pattern_hi[i] & 0x80) >> 7;
                    fg_pixel = (p1 << 1) | p0;
                    fg_palette = sprite.palette();
                    fg_priority = !sprite.attr().contains(SpriteAttributes::PRIORITY);

                    if fg_pixel != 0 {
                        if i == 0 {
                            // Sprite zero is visible
                            zero_visible = true;
                        }
                        break;
                    }
                }
            }
        }

        // Choose between foreground and background pixel
        let pixel: u8;
        let palette: u8;
        if (bg_pixel == 0) && (fg_pixel == 0) {
            pixel = 0x00;
            palette = 0x00;
        } else if (bg_pixel == 0) && (fg_pixel > 0) {
            pixel = fg_pixel;
            palette = fg_palette;
        } else if (bg_pixel > 0) && (fg_pixel == 0) {
            pixel = bg_pixel;
            palette = bg_palette;
        } else {
            if fg_priority {
                pixel = fg_pixel;
                palette = fg_palette;
            } else {
                pixel = bg_pixel;
                palette = bg_palette;
            }

            if self.allow_zero_hit && zero_visible {
                if self
                    .mask
                    .contains(PpuMask::RENDER_BACKGROUND | PpuMask::RENDER_SPRITES)
                {
                    if !self
                        .mask
                        .contains(PpuMask::RENDER_BACKGROUND_LEFT | PpuMask::RENDER_SPRITES_LEFT)
                    {
                        if (self.cycle > 8) && (self.cycle < 258) {
                            self.status.insert(PpuStatus::SPRITE_ZERO_HIT);
                        }
                    } else {
                        if (self.cycle > 0) && (self.cycle < 258) {
                            self.status.insert(PpuStatus::SPRITE_ZERO_HIT);
                        }
                    }
                }
            }
        }

        let x = (self.cycle as isize) - 1;
        let y = self.scanline as isize;
        let color = self.get_palette_color(Address::new(palette as u16), Wrapping(pixel));
        if (x >= 0) && (y >= 0) && (x < SCREEN_WIDTH as isize) && (y < SCREEN_HEIGHT as isize) {
            self.back_buffer.set_pixel(x as usize, y as usize, color);
        }

        self.cycle += 1;

        if self
            .mask
            .intersects(PpuMask::RENDER_BACKGROUND | PpuMask::RENDER_SPRITES)
        {
            if (self.cycle == 260) && (self.scanline < VBLANK_LINE) {
                if let Some(cartridge) = &self.cartridge {
                    let mut cart = cartridge.borrow_mut();
                    cart.on_scanline();
                    std::mem::drop(cart);
                }
            }
        }

        if self.cycle > MAX_CYCLE {
            self.cycle = 0;
            self.scanline += 1;
            if self.scanline > MAX_SCANLINE {
                self.scanline = -1;
                std::mem::swap(&mut self.back_buffer, &mut self.front_buffer);
            }
        }
    }

    #[inline]
    pub fn dma_write(&mut self, addr: Wrapping<u8>, data: Word) {
        self.oam.write(addr, data);
    }
}
impl<'a> BusComponent<cpu::cpu6502::Address, cpu::cpu6502::Word> for Ppu2C02<'a> {
    #[inline]
    fn read_range(&self) -> Option<AddressRange<cpu::cpu6502::Address>> {
        Some(self.range)
    }
    #[inline]
    fn write_range(&self) -> Option<AddressRange<cpu::cpu6502::Address>> {
        Some(self.range)
    }

    fn read(&mut self, addr: cpu::cpu6502::Address) -> cpu::cpu6502::Word {
        match addr {
            ADDR_CONTROL => Wrapping(0), // Not readable
            ADDR_MASK => Wrapping(0),    // Not readable
            ADDR_STATUS => {
                // The unused bytes contain the last buffer data on real hardware
                let tmp =
                    Wrapping(self.status.bits() & 0xE0) | (self.ppu_data_buffer & Wrapping(0x1F));
                self.status.remove(PpuStatus::VERTICAL_BLANK);
                self.ppu_addr_latch = false;
                tmp
            }
            ADDR_OAM_ADDRESS => Wrapping(0), // Not readable
            ADDR_OAM_DATA => self.oam.read(self.oam_addr),
            ADDR_SCROLL => Wrapping(0),      // Not readable
            ADDR_PPU_ADDRESS => Wrapping(0), // Not readable
            ADDR_PPU_DATA => {
                // Everything except palette data is buffered one cycle
                let mut tmp = self.ppu_data_buffer;
                self.ppu_data_buffer = self.read_bus(Address::new(self.vram_addr.value));
                if self.vram_addr.value >= 0x3F00 {
                    tmp = self.ppu_data_buffer;
                }
                // Auto-increment
                self.vram_addr.value +=
                    select(self.control.contains(PpuControl::INCREMENT_MODE), 32, 1);
                self.vram_addr.update_subfields();
                tmp
            }
            _ => Wrapping(0),
        }
    }

    fn write(&mut self, addr: cpu::cpu6502::Address, data: cpu::cpu6502::Word) {
        match addr {
            ADDR_CONTROL => {
                self.control = PpuControl::from_bits_truncate(data.0);
                self.tram_addr.nametable_x =
                    select(self.control.contains(PpuControl::NAMETABLE_X), 1, 0);
                self.tram_addr.nametable_y =
                    select(self.control.contains(PpuControl::NAMETABLE_Y), 1, 0);
                self.tram_addr.update_value();
            }
            ADDR_MASK => self.mask = PpuMask::from_bits_truncate(data.0),
            ADDR_STATUS => {} // Cannot write to status register
            ADDR_OAM_ADDRESS => self.oam_addr = data,
            ADDR_OAM_DATA => self.oam.write(self.oam_addr, data),
            ADDR_SCROLL => {
                if self.ppu_addr_latch {
                    self.tram_addr.fine_y = (data.0 & 0x07) as u16;
                    self.tram_addr.coarse_y = (data.0 >> 3) as u16;
                } else {
                    self.fine_x = data.0 & 0x07;
                    self.tram_addr.coarse_x = (data.0 >> 3) as u16;
                }
                self.tram_addr.update_value();
                self.ppu_addr_latch = !self.ppu_addr_latch;
            }
            ADDR_PPU_ADDRESS => {
                if self.ppu_addr_latch {
                    self.tram_addr.value = (self.tram_addr.value & 0xFF00) | (data.0 as u16);
                    self.tram_addr.update_subfields();
                    self.vram_addr = self.tram_addr;
                } else {
                    self.tram_addr.value =
                        (self.tram_addr.value & 0x00FF) | (((data.0 & 0x3F) as u16) << 8);
                    self.tram_addr.update_subfields();
                }
                self.ppu_addr_latch = !self.ppu_addr_latch;
            }
            ADDR_PPU_DATA => {
                self.write_bus(Address::new(self.vram_addr.value), data);
                // Auto-increment
                self.vram_addr.value +=
                    select(self.control.contains(PpuControl::INCREMENT_MODE), 32, 1);
                self.vram_addr.update_subfields();
            }
            _ => {}
        }
    }
}
impl<'a> VideoChip<'a, cpu::cpu6502::Address, cpu::cpu6502::Word, Address, Word> for Ppu2C02<'a> {
    #[inline]
    fn get_buffer(&self) -> &(dyn VideoBuffer + 'a) {
        self.front_buffer.as_ref()
    }

    fn reset(&mut self) {
        self.fine_x = 0;
        self.ppu_addr_latch = false;
        self.ppu_data_buffer = Wrapping(0);
        self.scanline = 0;
        self.cycle = 0;
        self.bg_next_id = 0;
        self.bg_next_attr = 0;
        self.bg_next_lsb = 0;
        self.bg_next_msb = 0;
        self.bg_pattern_lo.value = 0;
        self.bg_pattern_hi.value = 0;
        self.bg_attr_lo.value = 0;
        self.bg_attr_hi.value = 0;
        self.status = PpuStatus::empty();
        self.mask = PpuMask::empty();
        self.control = PpuControl::empty();
        self.vram_addr = PpuRegister::new();
        self.tram_addr = PpuRegister::new();
    }

    fn clock(&mut self, cycles: u32) {
        for _ in 0..cycles {
            self.clock_one();
        }
    }
}
