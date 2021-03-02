use crate::bus::Bus;
use crate::cpu::*;
use crate::types::*;
use std::num::Wrapping;
use std::ops::{Deref, DerefMut};
use strum_macros::{AsRefStr, Display, IntoStaticStr};

pub type Address = u24w;
pub type Word = u16w;
pub type HalfWord = u8w;

#[repr(C)]
union Register {
    lo_hi: [HalfWord; 2],
    full: Word,
}
impl Register {
    #[inline]
    const fn new() -> Self {
        Self { full: Wrapping(0) }
    }

    #[inline]
    fn lo(&self) -> &HalfWord {
        unsafe { &self.lo_hi[0] }
    }
    #[inline]
    fn hi(&self) -> &HalfWord {
        unsafe { &self.lo_hi[1] }
    }

    #[inline]
    fn lo_mut(&mut self) -> &mut HalfWord {
        unsafe { &mut self.lo_hi[0] }
    }
    #[inline]
    fn hi_mut(&mut self) -> &mut HalfWord {
        unsafe { &mut self.lo_hi[1] }
    }
}
impl Deref for Register {
    type Target = Word;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { &self.full }
    }
}
impl DerefMut for Register {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut self.full }
    }
}

bitflags! {
    struct StatusFlags : u8 {
        /// Carry
        const C = 0b00000001;
        /// Zero
        const Z = 0b00000010;
        /// IRQ disable
        const I = 0b00000100;
        /// Decimal mode
        const D = 0b00001000;
        /// Index register select
        const X = 0b00010000;
        /// Memory select
        const M = 0b00100000;
        /// Overflow
        const V = 0b01000000;
        /// Negative
        const N = 0b10000000;

        /*
            Flags in emulation mode
        */

        /// Break
        const B = 0b00010000;
        /// Unused
        const U = 0b00100000;
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Display, AsRefStr, IntoStaticStr)]
pub enum AddressingMode {}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Display, AsRefStr, IntoStaticStr)]
pub enum BaseInstruction {}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Instruction(BaseInstruction, AddressingMode, u32, bool);
impl CpuInstruction for Instruction {}

pub struct Cpu65C816<'a> {
    /// Accumulator
    c: Register,
    /// X index register
    x: Register,
    /// Y index register
    y: Register,
    /// Data bank
    db: HalfWord,
    /// Stack pointer
    sp: Register,
    /// Program counter
    pc: Word,
    /// Program bank
    pb: HalfWord,
    /// Direct register
    d: Register,
    /// Status register
    status: StatusFlags,
    /// Emulation mode flag
    emulation_mode: bool,

    bus: EmuRef<Bus<'a, Address, HalfWord>>,
}
impl<'a> Cpu65C816<'a> {
    pub fn new(bus: EmuRef<Bus<'a, Address, HalfWord>>) -> Self {
        Self {
            c: Register::new(),
            x: Register::new(),
            y: Register::new(),
            db: Wrapping(0),
            sp: Register::new(),
            pc: Wrapping(0),
            pb: Wrapping(0),
            d: Register::new(),
            status: StatusFlags::empty(),
            emulation_mode: false,
            bus,
        }
    }

    #[inline]
    fn a(&self) -> &HalfWord {
        self.c.lo()
    }
    #[inline]
    fn b(&self) -> &HalfWord {
        self.c.hi()
    }

    #[inline]
    fn a_mut(&mut self) -> &mut HalfWord {
        self.c.lo_mut()
    }
    #[inline]
    fn b_mut(&mut self) -> &mut HalfWord {
        self.c.hi_mut()
    }
}
