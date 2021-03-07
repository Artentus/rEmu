use crate::bus::Bus;
use crate::cpu::*;
use crate::types::*;
use std::num::Wrapping;
use std::ops::{Deref, DerefMut};
use strum_macros::{AsRefStr, Display, IntoStaticStr};

pub type Address = u24w;
pub type Word = u16w;
pub type Byte = u8w;

#[repr(C)]
union Register {
    lo_hi: [Byte; 2],
    full: Word,
}
impl Register {
    #[inline]
    const fn new() -> Self {
        Self { full: Wrapping(0) }
    }

    #[inline]
    fn lo(&self) -> &Byte {
        unsafe { &self.lo_hi[0] }
    }
    #[inline]
    fn hi(&self) -> &Byte {
        unsafe { &self.lo_hi[1] }
    }

    #[inline]
    fn lo_mut(&mut self) -> &mut Byte {
        unsafe { &mut self.lo_hi[0] }
    }
    #[inline]
    fn hi_mut(&mut self) -> &mut Byte {
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

#[derive(PartialEq, Eq, Clone, Copy, Debug, strum_macros::Display, AsRefStr, IntoStaticStr)]
enum AddressingMode {
    /// Implied
    IMP,
    /// Immediate byte
    IMB,
    /// Immediate word
    IMW,
    /// Zero-page
    ZP0,
    /// Zero-page + relative offset
    ZPR,
    /// Zero-page + X register offset
    ZPX,
    /// Zero-page + Y register offset
    ZPY,
    /// Relative byte
    REB,
    /// Relative word
    REW,
    /// Absolute
    ABS,
    /// Absolute long
    ABL,
    /// Absolute + X register offset
    ABX,
    /// Absolute + Y register offset
    ABY,
    /// Absolute long + X register offset
    ALX,
    /// Indirect
    IND,
    /// Indirect long
    INL,
    /// Indirect zero page
    IZP,
    /// Indirect (zero-page + X register offset)
    IZX,
    /// (Indirect zero-page) + Y register offset
    IZY,
    /// Indirect + X register offset
    IAX,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Display, AsRefStr, IntoStaticStr)]
pub enum BaseInstruction {}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Instruction(BaseInstruction, AddressingMode, u32, bool);

#[derive(Clone, Copy, Debug)]
pub struct Asm65C816Instruction {}
impl Display for Asm65C816Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}
impl AsmInstruction<Address> for Asm65C816Instruction {
    fn address(&self) -> Address {
        todo!()
    }

    fn byte_size(&self) -> usize {
        todo!()
    }

    fn mnemonic(&self) -> &str {
        todo!()
    }
}

const COP_VECTOR_NAT: Address = Address::new(0xFFE4);
const BRK_VECTOR_NAT: Address = Address::new(0xFFE6);
const ABORT_VECTOR_NAT: Address = Address::new(0xFFE8);
const NMI_VECTOR_NAT: Address = Address::new(0xFFEA);
const IRQ_VECTOR_NAT: Address = Address::new(0xFFEE);

const COP_VECTOR_EMU: Address = Address::new(0xFFF4);
const ABORT_VECTOR_EMU: Address = Address::new(0xFFF8);
const NMI_VECTOR_EMU: Address = Address::new(0xFFFA);
const IRQ_BRK_VECTOR_EMU: Address = Address::new(0xFFFE);

const RESET_VECTOR: Address = Address::new(0xFFFC);

pub struct Cpu65C816<'a> {
    /// Accumulator
    a: Register,
    /// X index register
    x: Register,
    /// Y index register
    y: Register,
    /// Stack pointer
    sp: Register,
    /// Direct page register
    dp: Register,
    /// Data bank
    db: Byte,
    /// Program bank
    pb: Byte,
    /// Program counter
    pc: Word,
    /// Status register
    status: StatusFlags,
    /// Emulation mode flag
    emulation_mode: bool,

    bus: EmuRef<Bus<'a, Address, Byte>>,
}
impl<'a> Cpu65C816<'a> {
    pub fn new(bus: EmuRef<Bus<'a, Address, Byte>>) -> Self {
        Self {
            a: Register::new(),
            x: Register::new(),
            y: Register::new(),
            sp: Register::new(),
            dp: Register::new(),
            db: Wrapping(0),
            pb: Wrapping(0),
            pc: Wrapping(0),
            status: StatusFlags::empty(),
            emulation_mode: false,
            bus,
        }
    }
}
impl<'a> Display for Cpu65C816<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}
impl<'a> Cpu<Address, Byte, Asm65C816Instruction> for Cpu65C816<'a> {
    fn reset(&mut self) -> u32 {
        *self.a = Wrapping(0);
        *self.x = Wrapping(0);
        *self.y = Wrapping(0);
        self.emulation_mode = true;

        8
    }

    fn execute_next_instruction(&mut self) -> u32 {
        todo!()
    }

    fn disassemble_current(&self, range: usize) -> Box<[Asm65C816Instruction]> {
        todo!()
    }
}
