use crate::bus::Bus;
use crate::cpu::*;
use std::num::Wrapping;
use strum_macros::{AsRefStr, Display, IntoStaticStr};

pub type Address = Wrapping<u16>;
pub type Word = Wrapping<u8>;

bitflags! {
    struct StatusFlags : u8 {
        /// Carry
        const C = 0b00000001;
        /// Zero
        const Z = 0b00000010;
        /// Interrupt
        const I = 0b00000100;
        /// Decimal
        const D = 0b00001000;
        /// Break
        const B = 0b00010000;
        /// Unused
        const U = 0b00100000;
        /// Overflow
        const V = 0b01000000;
        /// Negative
        const N = 0b10000000;
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Display, AsRefStr, IntoStaticStr)]
pub enum AddressingMode {
    /// Implied
    IMP = 0,
    /// Immediate
    IMM = 1,
    /// Zero-page
    ZP0 = 2,
    /// Zero-page + X register offset
    ZPX = 3,
    /// Zero-page + Y register offset
    ZPY = 4,
    /// Relative
    REL = 5,
    /// Absolute
    ABS = 6,
    /// Absolute + X register offset
    ABX = 7,
    /// Absolute + Y register offset
    ABY = 8,
    /// Indirect
    IND = 9,
    /// Indirect (zero-page + X register offset)
    IZX = 10,
    /// (Indirect zero-page) + Y register offset
    IZY = 11,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Display, AsRefStr, IntoStaticStr)]
pub enum BaseInstruction {
    LDA = 0,
    LDX = 1,
    LDY = 2,
    STA = 3,
    STX = 4,
    STY = 5,
    TAX = 6,
    TAY = 7,
    TXA = 8,
    TYA = 9,
    TSX = 10,
    TXS = 11,
    PHA = 12,
    PHP = 13,
    PLA = 14,
    PLP = 15,
    AND = 16,
    EOR = 17,
    ORA = 18,
    BIT = 19,
    ADC = 20,
    SBC = 21,
    CMP = 22,
    CPX = 23,
    CPY = 24,
    INC = 25,
    INX = 26,
    INY = 27,
    DEC = 28,
    DEX = 29,
    DEY = 30,
    ASL = 31,
    LSR = 32,
    ROL = 33,
    ROR = 34,
    JMP = 35,
    JSR = 36,
    RTS = 37,
    BCC = 38,
    BCS = 39,
    BEQ = 40,
    BMI = 41,
    BNE = 42,
    BPL = 43,
    BVC = 44,
    BVS = 45,
    CLC = 46,
    CLD = 47,
    CLI = 48,
    CLV = 49,
    SEC = 50,
    SED = 51,
    SEI = 52,
    BRK = 53,
    NOP = 54,
    RTI = 55,

    // Undocumented instructions
    SLO = 56,
    ANC = 57,
    RLA = 58,
    SRE = 59,
    ALR = 60,
    RRA = 61,
    ARR = 62,
    SAX = 63,
    XAA = 64,
    AHX = 65,
    TAS = 66,
    SHY = 67,
    SHX = 68,
    LAX = 69,
    LAS = 70,
    DCP = 71,
    AXS = 72,
    ISC = 73,
    HLT = 74,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Instruction(BaseInstruction, AddressingMode, u32);
impl crate::cpu::CpuInstruction for Instruction {}

#[derive(Debug)]
enum InstructionData {
    None,
    Data(Word),
    ZeroPageAddress(Word),
    AbsoluteAddress(Address),
}
impl InstructionData {
    fn read_data(&self, cpu: &Cpu6502) -> Word {
        match self {
            Self::Data(data) => *data,
            Self::ZeroPageAddress(address) => cpu.read_word(Wrapping(address.0 as u16)),
            Self::AbsoluteAddress(address) => cpu.read_word(*address),
            _ => panic!("Invalid addressing mode"),
        }
    }

    fn write_data(&self, cpu: &Cpu6502, data: Word) {
        match self {
            Self::ZeroPageAddress(address) => cpu.write_word(Wrapping(address.0 as u16), data),
            Self::AbsoluteAddress(address) => cpu.write_word(*address, data),
            _ => panic!("Invalid addressing mode"),
        };
    }

    fn read_address(&self) -> Address {
        match self {
            Self::ZeroPageAddress(address) => Wrapping(address.0 as u16),
            Self::AbsoluteAddress(address) => *address,
            _ => panic!("Invalid addressing mode"),
        }
    }
}

struct CycleInfo(bool, bool);

const STACK_BASE: Address = Wrapping(0x0100); // Stack base address
const IRQ_VECTOR: Address = Wrapping(0xFFFE); // Where to load the program counter from when an interrupt occurs
const NMI_VECTOR: Address = Wrapping(0xFFFA); // Where to load the program counter from when a non-maskable interrupt occurs
const RESET_VECTOR: Address = Wrapping(0xFFFC); // Where to load the program counter from when a reset occurs
const SP_INIT: Word = Wrapping(0xFD); // The initial top of the stack

pub struct Cpu6502<'a> {
    a: Word,
    x: Word,
    y: Word,
    sp: Word,
    pc: Address,
    status: StatusFlags,

    bus: EmuRef<Bus<'a, Address, Word>>,
}
impl<'a> Cpu6502<'a> {
    pub const fn new(bus: EmuRef<Bus<'a, Address, Word>>) -> Self {
        Self {
            a: Wrapping(0),
            x: Wrapping(0),
            y: Wrapping(0),
            sp: Wrapping(0),
            pc: Wrapping(0),
            status: StatusFlags::empty(),
            bus,
        }
    }

    #[inline]
    pub fn create(bus: EmuRef<Bus<'a, Address, Word>>) -> EmuRef<Self> {
        make_ref(Self::new(bus))
    }

    fn read_next_word(&mut self) -> Word {
        let bus_borrow = self.bus.borrow();
        let result = bus_borrow.read(self.pc);
        self.pc += Wrapping(1);
        result
    }

    fn read_next_address(&mut self) -> Address {
        let bus_borrow = self.bus.borrow();
        let lo = bus_borrow.read(self.pc);
        self.pc += Wrapping(1);
        let hi = bus_borrow.read(self.pc);
        self.pc += Wrapping(1);
        Wrapping((lo.0 as u16) | ((hi.0 as u16) << 8))
    }

    #[inline]
    fn read_word(&self, address: Address) -> Word {
        let bus_borrow = self.bus.borrow();
        bus_borrow.read(address)
    }

    fn read_address(&self, address: Address) -> Address {
        let bus_borrow = self.bus.borrow();
        let lo = bus_borrow.read(address + Wrapping(0));
        let hi = bus_borrow.read(address + Wrapping(1));
        Wrapping((lo.0 as u16) | ((hi.0 as u16) << 8))
    }

    fn read_address_ind(&self, address: Address) -> Address {
        let bus_borrow = self.bus.borrow();

        // Bug in the original hardware
        let page = address & Wrapping(0xFF00);
        let hi_address = ((address + Wrapping(1)) & Wrapping(0x00FF)) | page;

        let lo = bus_borrow.read(address);
        let hi = bus_borrow.read(hi_address);
        Wrapping((lo.0 as u16) | ((hi.0 as u16) << 8))
    }

    #[inline]
    fn write_word(&self, address: Address, data: Word) {
        let bus_borrow = self.bus.borrow();
        bus_borrow.write(address, data);
    }

    #[inline]
    fn set_zn_flags(&mut self, value: Word) {
        self.status.set(StatusFlags::Z, value.0 == 0);
        self.status.set(StatusFlags::N, (value.0 & 0x80) != 0);
    }

    #[inline]
    fn push_word(&mut self, data: Word) {
        let address = STACK_BASE + Wrapping(self.sp.0 as u16);
        self.sp -= Wrapping(1);
        self.write_word(address, data);
    }

    #[inline]
    fn pop_word(&mut self) -> Word {
        self.sp += Wrapping(1);
        let address = STACK_BASE + Wrapping(self.sp.0 as u16);
        self.read_word(address)
    }

    fn push_address(&mut self, data: Address) {
        let hi = Wrapping(((data.0 & 0xFF00) >> 8) as u8);
        let lo = Wrapping((data.0 & 0x00FF) as u8);
        self.push_word(hi);
        self.push_word(lo);
    }

    fn pop_address(&mut self) -> Address {
        let lo = self.pop_word();
        let hi = self.pop_word();
        Wrapping((lo.0 as u16) | ((hi.0 as u16) << 8))
    }

    #[inline]
    fn read_next_instruction(&mut self) -> Instruction {
        let op_code = self.read_next_word().0 as usize;
        INSTRUCTION_LOOKUP[op_code]
    }

    pub fn irq(&mut self) -> u32 {
        if !self.status.contains(StatusFlags::I) {
            self.status.remove(StatusFlags::B);
            self.status.insert(StatusFlags::U | StatusFlags::I);

            self.push_address(self.pc);
            self.push_word(Wrapping(self.status.bits()));

            self.pc = self.read_address(IRQ_VECTOR);

            7
        } else {
            0
        }
    }

    pub fn nmi(&mut self) -> u32 {
        self.status.remove(StatusFlags::B);
        self.status.insert(StatusFlags::U | StatusFlags::I);

        self.push_address(self.pc);
        self.push_word(Wrapping(self.status.bits()));

        self.pc = self.read_address(NMI_VECTOR);

        8
    }
}
impl<'a> Cpu<Address, Word, Instruction> for Cpu6502<'a> {
    fn reset(&mut self) -> u32 {
        self.a = Wrapping(0);
        self.x = Wrapping(0);
        self.y = Wrapping(0);
        self.sp = SP_INIT;
        self.status = StatusFlags::U;
        self.pc = self.read_address(RESET_VECTOR);

        8
    }

    fn execute_next_instruction(&mut self) -> u32 {
        let instruction = self.read_next_instruction();
        let base_instruction = instruction.0;
        let addressing_mode = instruction.1;
        let cycles = instruction.2;

        let addressing = ADDRESSING_LOOKUP[addressing_mode as usize](self);
        let instruction_data = addressing.0;
        let adds_cycle = addressing.1;

        let execute = EXECUTE_LOOKUP[base_instruction as usize];
        let cycle_info = execute(self, instruction_data);
        let can_add_cycle = cycle_info.0;
        let add_branch_cycle = cycle_info.1;

        cycles
            + if adds_cycle && can_add_cycle { 1 } else { 0 }
            + if add_branch_cycle { 1 } else { 0 }
    }
}

#[inline]
fn addressing_imp(_: &mut Cpu6502) -> (InstructionData, bool) {
    (InstructionData::None, false)
}

#[inline]
fn addressing_imm(cpu: &mut Cpu6502) -> (InstructionData, bool) {
    (InstructionData::Data(cpu.read_next_word()), false)
}

#[inline]
fn addressing_zp0(cpu: &mut Cpu6502) -> (InstructionData, bool) {
    (
        InstructionData::ZeroPageAddress(cpu.read_next_word()),
        false,
    )
}

#[inline]
fn addressing_zpx(cpu: &mut Cpu6502) -> (InstructionData, bool) {
    (
        InstructionData::ZeroPageAddress(cpu.read_next_word() + cpu.x),
        false,
    )
}

#[inline]
fn addressing_zpy(cpu: &mut Cpu6502) -> (InstructionData, bool) {
    (
        InstructionData::ZeroPageAddress(cpu.read_next_word() + cpu.y),
        false,
    )
}

fn addressing_rel(cpu: &mut Cpu6502) -> (InstructionData, bool) {
    let mut address = cpu.read_next_word().0 as u16;
    // Handle the negative case
    if (address & 0x0080) != 0 {
        address |= 0xFF00;
    }

    let abs_address = cpu.pc + Wrapping(address);
    let page_before = cpu.pc & Wrapping(0xFF00);
    let page_after = abs_address & Wrapping(0xFF00);
    let adds_cycle = if page_before != page_after {
        true
    } else {
        false
    };

    (InstructionData::AbsoluteAddress(abs_address), adds_cycle)
}

#[inline]
fn addressing_abs(cpu: &mut Cpu6502) -> (InstructionData, bool) {
    (
        InstructionData::AbsoluteAddress(cpu.read_next_address()),
        false,
    )
}

fn addressing_abx(cpu: &mut Cpu6502) -> (InstructionData, bool) {
    let address_before = cpu.read_next_address();
    let page_before = address_before & Wrapping(0xFF00);

    let address_after = address_before + Wrapping(cpu.x.0 as u16);
    let page_after = address_after & Wrapping(0xFF00);

    let adds_cycle = if page_before != page_after {
        true
    } else {
        false
    };
    (InstructionData::AbsoluteAddress(address_after), adds_cycle)
}

fn addressing_aby(cpu: &mut Cpu6502) -> (InstructionData, bool) {
    let address_before = cpu.read_next_address();
    let page_before = address_before & Wrapping(0xFF00);

    let address_after = address_before + Wrapping(cpu.y.0 as u16);
    let page_after = address_after & Wrapping(0xFF00);

    let adds_cycle = if page_before != page_after {
        true
    } else {
        false
    };
    (InstructionData::AbsoluteAddress(address_after), adds_cycle)
}

#[inline]
fn addressing_ind(cpu: &mut Cpu6502) -> (InstructionData, bool) {
    let pointer = cpu.read_next_address();
    let address = cpu.read_address_ind(pointer);
    (InstructionData::AbsoluteAddress(address), false)
}

#[inline]
fn addressing_izx(cpu: &mut Cpu6502) -> (InstructionData, bool) {
    let pointer = Wrapping((cpu.read_next_word() + cpu.x).0 as u16);
    let address = cpu.read_address_ind(pointer);
    (InstructionData::AbsoluteAddress(address), false)
}

fn addressing_izy(cpu: &mut Cpu6502) -> (InstructionData, bool) {
    let pointer = Wrapping(cpu.read_next_word().0 as u16);
    let address_before = cpu.read_address_ind(pointer);
    let page_before = address_before & Wrapping(0xFF00);

    let address_after = address_before + Wrapping(cpu.y.0 as u16);
    let page_after = address_after & Wrapping(0xFF00);

    let adds_cycle = if page_before != page_after {
        true
    } else {
        false
    };
    (InstructionData::AbsoluteAddress(address_after), adds_cycle)
}

const ADDRESSING_LOOKUP: [fn(&mut Cpu6502) -> (InstructionData, bool); 12] = [
    addressing_imp, // IMP
    addressing_imm, // IMM
    addressing_zp0, // ZP0
    addressing_zpx, // ZPX
    addressing_zpy, // ZPY
    addressing_rel, // REL
    addressing_abs, // ABS
    addressing_abx, // ABX
    addressing_aby, // ABY
    addressing_ind, // IND
    addressing_izx, // IZX
    addressing_izy, // IZY
];

#[inline]
fn execute_lda(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    cpu.a = data.read_data(cpu);
    cpu.set_zn_flags(cpu.a);
    CycleInfo(true, false)
}

#[inline]
fn execute_ldx(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    cpu.x = data.read_data(cpu);
    cpu.set_zn_flags(cpu.x);
    CycleInfo(true, false)
}

#[inline]
fn execute_ldy(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    cpu.y = data.read_data(cpu);
    cpu.set_zn_flags(cpu.y);
    CycleInfo(true, false)
}

#[inline]
fn execute_sta(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    data.write_data(cpu, cpu.a);
    CycleInfo(false, false)
}

#[inline]
fn execute_stx(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    data.write_data(cpu, cpu.x);
    CycleInfo(false, false)
}

#[inline]
fn execute_sty(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    data.write_data(cpu, cpu.y);
    CycleInfo(false, false)
}

#[inline]
fn execute_tax(cpu: &mut Cpu6502, _: InstructionData) -> CycleInfo {
    cpu.x = cpu.a;
    cpu.set_zn_flags(cpu.x);
    CycleInfo(false, false)
}

#[inline]
fn execute_tay(cpu: &mut Cpu6502, _: InstructionData) -> CycleInfo {
    cpu.y = cpu.a;
    cpu.set_zn_flags(cpu.y);
    CycleInfo(false, false)
}

#[inline]
fn execute_txa(cpu: &mut Cpu6502, _: InstructionData) -> CycleInfo {
    cpu.a = cpu.x;
    cpu.set_zn_flags(cpu.a);
    CycleInfo(false, false)
}

#[inline]
fn execute_tya(cpu: &mut Cpu6502, _: InstructionData) -> CycleInfo {
    cpu.a = cpu.y;
    cpu.set_zn_flags(cpu.a);
    CycleInfo(false, false)
}

#[inline]
fn execute_tsx(cpu: &mut Cpu6502, _: InstructionData) -> CycleInfo {
    cpu.x = cpu.sp;
    cpu.set_zn_flags(cpu.x);
    CycleInfo(false, false)
}

#[inline]
fn execute_txs(cpu: &mut Cpu6502, _: InstructionData) -> CycleInfo {
    cpu.sp = cpu.x;
    CycleInfo(false, false)
}

#[inline]
fn execute_pha(cpu: &mut Cpu6502, _: InstructionData) -> CycleInfo {
    cpu.push_word(cpu.a);
    CycleInfo(false, false)
}

#[inline]
fn execute_php(cpu: &mut Cpu6502, _: InstructionData) -> CycleInfo {
    cpu.push_word(Wrapping(
        (cpu.status | StatusFlags::B | StatusFlags::U).bits(),
    ));
    cpu.status.remove(StatusFlags::B | StatusFlags::U);
    CycleInfo(false, false)
}

#[inline]
fn execute_pla(cpu: &mut Cpu6502, _: InstructionData) -> CycleInfo {
    cpu.a = cpu.pop_word();
    cpu.set_zn_flags(cpu.a);
    CycleInfo(false, false)
}

#[inline]
fn execute_plp(cpu: &mut Cpu6502, _: InstructionData) -> CycleInfo {
    unsafe {
        cpu.status = StatusFlags::from_bits_unchecked(cpu.pop_word().0);
    }
    cpu.status.insert(StatusFlags::U);
    CycleInfo(false, false)
}

#[inline]
fn execute_and(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    cpu.a &= data.read_data(cpu);
    cpu.set_zn_flags(cpu.a);
    CycleInfo(true, false)
}

#[inline]
fn execute_eor(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    cpu.a ^= data.read_data(cpu);
    cpu.set_zn_flags(cpu.a);
    CycleInfo(true, false)
}

#[inline]
fn execute_ora(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    cpu.a |= data.read_data(cpu);
    cpu.set_zn_flags(cpu.a);
    CycleInfo(true, false)
}

fn execute_bit(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    let value = data.read_data(cpu);
    cpu.status.set(StatusFlags::Z, (cpu.a & value).0 == 0);
    cpu.status.set(StatusFlags::N, (value.0 & 0x80) != 0);
    cpu.status.set(StatusFlags::V, (value.0 & 0x40) != 0);
    CycleInfo(false, false)
}

fn execute_adc_sbc(cpu: &mut Cpu6502, right: u16) -> CycleInfo {
    let left = cpu.a.0 as u16;
    let carry: u16 = if cpu.status.contains(StatusFlags::C) {
        1
    } else {
        0
    };
    let result = left + right + carry;

    let is_overflow = if ((!(left ^ right) & (left ^ result)) & 0x0080) != 0 {
        true
    } else {
        false
    };

    cpu.a = Wrapping((result & 0x00FF) as u8);
    cpu.status.set(StatusFlags::C, (result & 0xFF00) != 0);
    cpu.status.set(StatusFlags::V, is_overflow);
    cpu.set_zn_flags(cpu.a);
    CycleInfo(true, false)
}

#[inline]
fn execute_adc(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    let right = data.read_data(cpu).0 as u16;
    execute_adc_sbc(cpu, right)
}

#[inline]
fn execute_sbc(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    let right = (!data.read_data(cpu).0) as u16;
    execute_adc_sbc(cpu, right)
}

fn execute_cmp(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    let value = data.read_data(cpu);
    let tmp = cpu.a - value;
    cpu.status.set(StatusFlags::C, cpu.a >= value);
    cpu.set_zn_flags(tmp);
    CycleInfo(true, false)
}

fn execute_cpx(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    let value = data.read_data(cpu);
    let tmp = cpu.x - value;
    cpu.status.set(StatusFlags::C, cpu.x >= value);
    cpu.set_zn_flags(tmp);
    CycleInfo(false, false)
}

fn execute_cpy(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    let value = data.read_data(cpu);
    let tmp = cpu.y - value;
    cpu.status.set(StatusFlags::C, cpu.y >= value);
    cpu.set_zn_flags(tmp);
    CycleInfo(false, false)
}

#[inline]
fn execute_inc(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    let value = data.read_data(cpu) + Wrapping(1);
    data.write_data(cpu, value);
    cpu.set_zn_flags(value);
    CycleInfo(false, false)
}

#[inline]
fn execute_inx(cpu: &mut Cpu6502, _: InstructionData) -> CycleInfo {
    cpu.x += Wrapping(1);
    cpu.set_zn_flags(cpu.x);
    CycleInfo(false, false)
}

#[inline]
fn execute_iny(cpu: &mut Cpu6502, _: InstructionData) -> CycleInfo {
    cpu.y += Wrapping(1);
    cpu.set_zn_flags(cpu.y);
    CycleInfo(false, false)
}

#[inline]
fn execute_dec(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    let value = data.read_data(cpu) - Wrapping(1);
    data.write_data(cpu, value);
    cpu.set_zn_flags(value);
    CycleInfo(false, false)
}

#[inline]
fn execute_dex(cpu: &mut Cpu6502, _: InstructionData) -> CycleInfo {
    cpu.x -= Wrapping(1);
    cpu.set_zn_flags(cpu.x);
    CycleInfo(false, false)
}

#[inline]
fn execute_dey(cpu: &mut Cpu6502, _: InstructionData) -> CycleInfo {
    cpu.y -= Wrapping(1);
    cpu.set_zn_flags(cpu.y);
    CycleInfo(false, false)
}

fn execute_asl(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    if let InstructionData::None = data {
        // If no address is provided the operation is applied to the accumulator
        cpu.status.set(StatusFlags::C, (cpu.a.0 & 0x80) != 0);
        cpu.a <<= 1;
        cpu.set_zn_flags(cpu.a);
    } else {
        let value = data.read_data(cpu);
        cpu.status.set(StatusFlags::C, (value.0 & 0x80) != 0);

        let tmp = value << 1;
        cpu.set_zn_flags(tmp);
        data.write_data(cpu, tmp);
    }
    CycleInfo(false, false)
}

fn execute_lsr(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    if let InstructionData::None = data {
        // If no address is provided the operation is applied to the accumulator
        cpu.status.set(StatusFlags::C, (cpu.a.0 & 0x01) != 0);
        cpu.a >>= 1;
        cpu.set_zn_flags(cpu.a);
    } else {
        let value = data.read_data(cpu);
        cpu.status.set(StatusFlags::C, (value.0 & 0x01) != 0);

        let tmp = value >> 1;
        cpu.set_zn_flags(tmp);
        data.write_data(cpu, tmp);
    }
    CycleInfo(false, false)
}

fn execute_rol(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    if let InstructionData::None = data {
        // If no address is provided the operation is applied to the accumulator
        let tmp = ((cpu.a.0 as u16) << 1)
            | if cpu.status.contains(StatusFlags::C) {
                0x0001
            } else {
                0x0000
            };
        cpu.status.set(StatusFlags::C, (tmp & 0xFF00) != 0);
        cpu.a = Wrapping((tmp & 0x00FF) as u8);
        cpu.set_zn_flags(cpu.a);
    } else {
        let value = data.read_data(cpu);
        let tmp = ((value.0 as u16) << 1)
            | if cpu.status.contains(StatusFlags::C) {
                0x0001
            } else {
                0x0000
            };
        cpu.status.set(StatusFlags::C, (tmp & 0xFF00) != 0);

        let new_value = Wrapping((tmp & 0x00FF) as u8);
        cpu.set_zn_flags(new_value);
        data.write_data(cpu, new_value);
    }
    CycleInfo(false, false)
}

fn execute_ror(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    if let InstructionData::None = data {
        // If no address is provided the operation is applied to the accumulator
        let tmp = (cpu.a >> 1)
            | if cpu.status.contains(StatusFlags::C) {
                Wrapping(0x80)
            } else {
                Wrapping(0x00)
            };
        cpu.status.set(StatusFlags::C, (cpu.a.0 & 0x01) != 0);
        cpu.a = tmp;
        cpu.set_zn_flags(cpu.a);
    } else {
        let value = data.read_data(cpu);
        let tmp = (value >> 1)
            | if cpu.status.contains(StatusFlags::C) {
                Wrapping(0x80)
            } else {
                Wrapping(0x00)
            };
        cpu.status.set(StatusFlags::C, (value.0 & 0x01) != 0);
        data.write_data(cpu, tmp);
        cpu.set_zn_flags(tmp);
    }
    CycleInfo(false, false)
}

#[inline]
fn execute_jmp(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    cpu.pc = data.read_address();
    CycleInfo(false, false)
}

#[inline]
fn execute_jsr(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    cpu.pc -= Wrapping(1);
    cpu.push_address(cpu.pc);
    cpu.pc = data.read_address();
    CycleInfo(false, false)
}

#[inline]
fn execute_rts(cpu: &mut Cpu6502, _: InstructionData) -> CycleInfo {
    cpu.pc = cpu.pop_address() + Wrapping(1);
    CycleInfo(false, false)
}

#[inline]
fn execute_bcc(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    if !cpu.status.contains(StatusFlags::C) {
        cpu.pc = data.read_address();
        CycleInfo(true, true)
    } else {
        CycleInfo(false, false)
    }
}

#[inline]
fn execute_bcs(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    if cpu.status.contains(StatusFlags::C) {
        cpu.pc = data.read_address();
        CycleInfo(true, true)
    } else {
        CycleInfo(false, false)
    }
}

#[inline]
fn execute_beq(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    if cpu.status.contains(StatusFlags::Z) {
        cpu.pc = data.read_address();
        CycleInfo(true, true)
    } else {
        CycleInfo(false, false)
    }
}

#[inline]
fn execute_bmi(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    if cpu.status.contains(StatusFlags::N) {
        cpu.pc = data.read_address();
        CycleInfo(true, true)
    } else {
        CycleInfo(false, false)
    }
}

#[inline]
fn execute_bne(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    if !cpu.status.contains(StatusFlags::Z) {
        cpu.pc = data.read_address();
        CycleInfo(true, true)
    } else {
        CycleInfo(false, false)
    }
}

#[inline]
fn execute_bpl(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    if !cpu.status.contains(StatusFlags::N) {
        cpu.pc = data.read_address();
        CycleInfo(true, true)
    } else {
        CycleInfo(false, false)
    }
}

#[inline]
fn execute_bvc(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    if !cpu.status.contains(StatusFlags::V) {
        cpu.pc = data.read_address();
        CycleInfo(true, true)
    } else {
        CycleInfo(false, false)
    }
}

#[inline]
fn execute_bvs(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    if cpu.status.contains(StatusFlags::V) {
        cpu.pc = data.read_address();
        CycleInfo(true, true)
    } else {
        CycleInfo(false, false)
    }
}

#[inline]
fn execute_clc(cpu: &mut Cpu6502, _: InstructionData) -> CycleInfo {
    cpu.status.remove(StatusFlags::C);
    CycleInfo(false, false)
}

#[inline]
fn execute_cld(cpu: &mut Cpu6502, _: InstructionData) -> CycleInfo {
    cpu.status.remove(StatusFlags::D);
    CycleInfo(false, false)
}

#[inline]
fn execute_cli(cpu: &mut Cpu6502, _: InstructionData) -> CycleInfo {
    cpu.status.remove(StatusFlags::I);
    CycleInfo(false, false)
}

#[inline]
fn execute_clv(cpu: &mut Cpu6502, _: InstructionData) -> CycleInfo {
    cpu.status.remove(StatusFlags::V);
    CycleInfo(false, false)
}

#[inline]
fn execute_sec(cpu: &mut Cpu6502, _: InstructionData) -> CycleInfo {
    cpu.status.insert(StatusFlags::C);
    CycleInfo(false, false)
}

#[inline]
fn execute_sed(cpu: &mut Cpu6502, _: InstructionData) -> CycleInfo {
    cpu.status.insert(StatusFlags::D);
    CycleInfo(false, false)
}

#[inline]
fn execute_sei(cpu: &mut Cpu6502, _: InstructionData) -> CycleInfo {
    cpu.status.insert(StatusFlags::I);
    CycleInfo(false, false)
}

#[inline]
fn execute_brk(cpu: &mut Cpu6502, _: InstructionData) -> CycleInfo {
    cpu.pc += Wrapping(1);
    cpu.push_address(cpu.pc);

    cpu.status.insert(StatusFlags::B | StatusFlags::I);
    cpu.push_word(Wrapping(cpu.status.bits()));
    cpu.status.remove(StatusFlags::B);

    cpu.pc = cpu.read_address(IRQ_VECTOR);
    CycleInfo(false, false)
}

#[inline]
fn execute_nop(_: &mut Cpu6502, _: InstructionData) -> CycleInfo {
    CycleInfo(true, false)
}

#[inline]
fn execute_rti(cpu: &mut Cpu6502, _: InstructionData) -> CycleInfo {
    unsafe {
        cpu.status = StatusFlags::from_bits_unchecked(cpu.pop_word().0);
    }
    cpu.status.remove(StatusFlags::B | StatusFlags::U);
    cpu.pc = cpu.pop_address();
    CycleInfo(false, false)
}

fn execute_slo(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    let value = data.read_data(cpu);
    cpu.status.set(StatusFlags::C, (value.0 & 0x80) != 0);

    let tmp = value << 1;
    data.write_data(cpu, tmp);

    cpu.a |= tmp;
    cpu.set_zn_flags(cpu.a);

    CycleInfo(true, false)
}

#[inline]
fn execute_anc(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    cpu.status.set(StatusFlags::C, (cpu.a.0 & 0x80) != 0);
    cpu.a &= data.read_data(cpu);
    cpu.set_zn_flags(cpu.a);
    CycleInfo(true, false)
}

fn execute_rla(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    let value = data.read_data(cpu);
    let tmp = ((value.0 as u16) << 1)
        | if cpu.status.contains(StatusFlags::C) {
            0x0001
        } else {
            0x0000
        };
    cpu.status.set(StatusFlags::C, (tmp & 0xFF00) != 0);

    let new_value = Wrapping((tmp & 0x00FF) as u8);
    data.write_data(cpu, new_value);

    cpu.a &= new_value;
    cpu.set_zn_flags(cpu.a);

    CycleInfo(true, false)
}

fn execute_sre(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    let value = data.read_data(cpu);
    cpu.status.set(StatusFlags::C, (value.0 & 0x01) != 0);

    let tmp = value >> 1;
    data.write_data(cpu, tmp);

    cpu.a ^= tmp;
    cpu.set_zn_flags(cpu.a);

    CycleInfo(true, false)
}

#[inline]
fn execute_alr(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    cpu.a &= data.read_data(cpu);
    cpu.status.set(StatusFlags::C, (cpu.a.0 & 0x01) != 0);
    cpu.a >>= 1;
    cpu.set_zn_flags(cpu.a);
    CycleInfo(true, false)
}

fn execute_rra(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    let value = data.read_data(cpu);
    let tmp = (value >> 1)
        | if cpu.status.contains(StatusFlags::C) {
            Wrapping(0x80)
        } else {
            Wrapping(0x00)
        };
    cpu.status.set(StatusFlags::C, (value.0 & 0x01) != 0);
    data.write_data(cpu, tmp);

    let right = tmp.0 as u16;
    let result = execute_adc_sbc(cpu, right);

    result
}

fn execute_arr(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    cpu.a &= data.read_data(cpu);
    let tmp = (cpu.a >> 1)
        | if cpu.status.contains(StatusFlags::C) {
            Wrapping(0x80)
        } else {
            Wrapping(0x00)
        };
    cpu.a = tmp;
    cpu.set_zn_flags(cpu.a);
    CycleInfo(true, false)
}

#[inline]
fn execute_sax(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    data.write_data(cpu, cpu.a & cpu.x);
    CycleInfo(false, false)
}

#[inline]
fn execute_xaa(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    cpu.a = cpu.a & cpu.x & data.read_data(cpu);
    cpu.set_zn_flags(cpu.a);
    CycleInfo(false, false)
}

#[inline]
fn execute_ahx(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    data.write_data(cpu, cpu.a & cpu.x & data.read_data(cpu));
    CycleInfo(false, false)
}

#[inline]
fn execute_tas(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    cpu.sp = cpu.a & cpu.x;
    data.write_data(cpu, cpu.a & cpu.x & data.read_data(cpu));
    CycleInfo(false, false)
}

#[inline]
fn execute_shy(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    data.write_data(cpu, cpu.y & data.read_data(cpu));
    CycleInfo(false, false)
}

#[inline]
fn execute_shx(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    data.write_data(cpu, cpu.x & data.read_data(cpu));
    CycleInfo(false, false)
}

#[inline]
fn execute_lax(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    cpu.a = data.read_data(cpu);
    cpu.x = cpu.a;
    cpu.set_zn_flags(cpu.a);
    CycleInfo(true, false)
}

#[inline]
fn execute_las(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    cpu.a = data.read_data(cpu) & cpu.sp;
    cpu.x = cpu.a;
    cpu.sp = cpu.a;
    cpu.set_zn_flags(cpu.a);
    CycleInfo(false, false)
}

fn execute_dcp(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    let value = data.read_data(cpu) - Wrapping(1);
    data.write_data(cpu, value);

    let tmp = cpu.a - value;
    cpu.status.set(StatusFlags::C, cpu.a >= value);
    cpu.set_zn_flags(tmp);

    CycleInfo(true, false)
}

#[inline]
fn execute_axs(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    let value = data.read_data(cpu);
    let tmp = (cpu.a & cpu.x) - value;
    cpu.status.set(StatusFlags::C, (cpu.a & cpu.x) >= value);
    cpu.set_zn_flags(tmp);
    cpu.x = tmp;
    CycleInfo(true, false)
}

#[inline]
fn execute_isc(cpu: &mut Cpu6502, data: InstructionData) -> CycleInfo {
    let value = data.read_data(cpu) + Wrapping(1);
    data.write_data(cpu, value);

    let right = (!value.0) as u16;
    let result = execute_adc_sbc(cpu, right);

    result
}

#[inline]
fn execute_hlt(_: &mut Cpu6502, _: InstructionData) -> CycleInfo {
    panic!("Invalid instruction")
}

const EXECUTE_LOOKUP: [fn(&mut Cpu6502, InstructionData) -> CycleInfo; 75] = [
    execute_lda, // LDA
    execute_ldx, // LDX
    execute_ldy, // LDY
    execute_sta, // STA
    execute_stx, // STX
    execute_sty, // STY
    execute_tax, // TAX
    execute_tay, // TAY
    execute_txa, // TXA
    execute_tya, // TYA
    execute_tsx, // TSX
    execute_txs, // TXS
    execute_pha, // PHA
    execute_php, // PHP
    execute_pla, // PLA
    execute_plp, // PLP
    execute_and, // AND
    execute_eor, // EOR
    execute_ora, // ORA
    execute_bit, // BIT
    execute_adc, // ADC
    execute_sbc, // SBC
    execute_cmp, // CMP
    execute_cpx, // CPX
    execute_cpy, // CPY
    execute_inc, // INC
    execute_inx, // INX
    execute_iny, // INY
    execute_dec, // DEC
    execute_dex, // DEX
    execute_dey, // DEY
    execute_asl, // ASL
    execute_lsr, // LSR
    execute_rol, // ROL
    execute_ror, // ROR
    execute_jmp, // JMP
    execute_jsr, // JSR
    execute_rts, // RTS
    execute_bcc, // BCC
    execute_bcs, // BCS
    execute_beq, // BEQ
    execute_bmi, // BMI
    execute_bne, // BNE
    execute_bpl, // BPL
    execute_bvc, // BVC
    execute_bvs, // BVS
    execute_clc, // CLC
    execute_cld, // CLD
    execute_cli, // CLI
    execute_clv, // CLV
    execute_sec, // SEC
    execute_sed, // SED
    execute_sei, // SEI
    execute_brk, // BRK
    execute_nop, // NOP
    execute_rti, // RTI
    //
    execute_slo, // SLO
    execute_anc, // ANC
    execute_rla, // RLA
    execute_sre, // SRE
    execute_alr, // ALR
    execute_rra, // RRA
    execute_arr, // ARR
    execute_sax, // SAX
    execute_xaa, // XAA
    execute_ahx, // AHX
    execute_tas, // TAS
    execute_shy, // SHY
    execute_shx, // SHX
    execute_lax, // LAX
    execute_las, // LAS
    execute_dcp, // DCP
    execute_axs, // AXS
    execute_isc, // ISC
    execute_hlt, // HLT
];

const INSTRUCTION_LOOKUP: [Instruction; 256] = [
    Instruction(BaseInstruction::BRK, AddressingMode::IMP, 7), // 0x00
    Instruction(BaseInstruction::ORA, AddressingMode::IZX, 6), // 0x01
    Instruction(BaseInstruction::HLT, AddressingMode::IMP, 0), // 0x02
    Instruction(BaseInstruction::SLO, AddressingMode::IZX, 8), // 0x03
    Instruction(BaseInstruction::NOP, AddressingMode::ZP0, 3), // 0x04
    Instruction(BaseInstruction::ORA, AddressingMode::ZP0, 3), // 0x05
    Instruction(BaseInstruction::ASL, AddressingMode::ZP0, 5), // 0x06
    Instruction(BaseInstruction::SLO, AddressingMode::ZP0, 5), // 0x07
    Instruction(BaseInstruction::PHP, AddressingMode::IMP, 3), // 0x08
    Instruction(BaseInstruction::ORA, AddressingMode::IMM, 2), // 0x09
    Instruction(BaseInstruction::ASL, AddressingMode::IMP, 2), // 0x0A
    Instruction(BaseInstruction::ANC, AddressingMode::IMM, 2), // 0x0B
    Instruction(BaseInstruction::NOP, AddressingMode::ABS, 4), // 0x0C
    Instruction(BaseInstruction::ORA, AddressingMode::ABS, 4), // 0x0D
    Instruction(BaseInstruction::ASL, AddressingMode::ABS, 6), // 0x0E
    Instruction(BaseInstruction::SLO, AddressingMode::ABS, 6), // 0x0F
    //
    Instruction(BaseInstruction::BPL, AddressingMode::REL, 2), // 0x10
    Instruction(BaseInstruction::ORA, AddressingMode::IZY, 5), // 0x11
    Instruction(BaseInstruction::HLT, AddressingMode::IMP, 0), // 0x12
    Instruction(BaseInstruction::SLO, AddressingMode::IZY, 8), // 0x13
    Instruction(BaseInstruction::NOP, AddressingMode::ZPX, 4), // 0x14
    Instruction(BaseInstruction::ORA, AddressingMode::ZPX, 4), // 0x15
    Instruction(BaseInstruction::ASL, AddressingMode::ZPX, 6), // 0x16
    Instruction(BaseInstruction::SLO, AddressingMode::ZPX, 6), // 0x17
    Instruction(BaseInstruction::CLC, AddressingMode::IMP, 2), // 0x18
    Instruction(BaseInstruction::ORA, AddressingMode::ABY, 4), // 0x19
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 2), // 0x1A
    Instruction(BaseInstruction::SLO, AddressingMode::ABY, 7), // 0x1B
    Instruction(BaseInstruction::NOP, AddressingMode::ABX, 4), // 0x1C
    Instruction(BaseInstruction::ORA, AddressingMode::ABX, 4), // 0x1D
    Instruction(BaseInstruction::ASL, AddressingMode::ABX, 7), // 0x1E
    Instruction(BaseInstruction::SLO, AddressingMode::ABX, 7), // 0x1F
    //
    Instruction(BaseInstruction::JSR, AddressingMode::ABS, 6), // 0x20
    Instruction(BaseInstruction::AND, AddressingMode::IZX, 6), // 0x21
    Instruction(BaseInstruction::HLT, AddressingMode::IMP, 0), // 0x22
    Instruction(BaseInstruction::RLA, AddressingMode::IZX, 8), // 0x23
    Instruction(BaseInstruction::BIT, AddressingMode::ZP0, 3), // 0x24
    Instruction(BaseInstruction::AND, AddressingMode::ZP0, 3), // 0x25
    Instruction(BaseInstruction::ROL, AddressingMode::ZP0, 5), // 0x26
    Instruction(BaseInstruction::RLA, AddressingMode::ZP0, 5), // 0x27
    Instruction(BaseInstruction::PLP, AddressingMode::IMP, 4), // 0x28
    Instruction(BaseInstruction::AND, AddressingMode::IMM, 2), // 0x29
    Instruction(BaseInstruction::ROL, AddressingMode::IMP, 2), // 0x2A
    Instruction(BaseInstruction::ANC, AddressingMode::IMM, 2), // 0x2B
    Instruction(BaseInstruction::BIT, AddressingMode::ABS, 4), // 0x2C
    Instruction(BaseInstruction::AND, AddressingMode::ABS, 4), // 0x2D
    Instruction(BaseInstruction::ROL, AddressingMode::ABS, 6), // 0x2E
    Instruction(BaseInstruction::RLA, AddressingMode::ABS, 6), // 0x2F
    //
    Instruction(BaseInstruction::BMI, AddressingMode::REL, 2), // 0x30
    Instruction(BaseInstruction::AND, AddressingMode::IZY, 5), // 0x31
    Instruction(BaseInstruction::HLT, AddressingMode::IMP, 0), // 0x32
    Instruction(BaseInstruction::RLA, AddressingMode::IZY, 8), // 0x33
    Instruction(BaseInstruction::NOP, AddressingMode::ZPX, 4), // 0x34
    Instruction(BaseInstruction::AND, AddressingMode::ZPX, 4), // 0x35
    Instruction(BaseInstruction::ROL, AddressingMode::ZPX, 6), // 0x36
    Instruction(BaseInstruction::RLA, AddressingMode::ZPX, 6), // 0x37
    Instruction(BaseInstruction::SEC, AddressingMode::IMP, 2), // 0x38
    Instruction(BaseInstruction::AND, AddressingMode::ABY, 4), // 0x39
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 2), // 0x3A
    Instruction(BaseInstruction::RLA, AddressingMode::ABY, 7), // 0x3B
    Instruction(BaseInstruction::NOP, AddressingMode::ABX, 4), // 0x3C
    Instruction(BaseInstruction::AND, AddressingMode::ABX, 4), // 0x3D
    Instruction(BaseInstruction::ROL, AddressingMode::ABX, 7), // 0x3E
    Instruction(BaseInstruction::RLA, AddressingMode::ABX, 7), // 0x3F
    //
    Instruction(BaseInstruction::RTI, AddressingMode::IMP, 6), // 0x40
    Instruction(BaseInstruction::EOR, AddressingMode::IZX, 6), // 0x41
    Instruction(BaseInstruction::HLT, AddressingMode::IMP, 0), // 0x42
    Instruction(BaseInstruction::SRE, AddressingMode::IZX, 8), // 0x43
    Instruction(BaseInstruction::NOP, AddressingMode::ZP0, 3), // 0x44
    Instruction(BaseInstruction::EOR, AddressingMode::ZP0, 3), // 0x45
    Instruction(BaseInstruction::LSR, AddressingMode::ZP0, 5), // 0x46
    Instruction(BaseInstruction::SRE, AddressingMode::ZP0, 5), // 0x47
    Instruction(BaseInstruction::PHA, AddressingMode::IMP, 3), // 0x48
    Instruction(BaseInstruction::EOR, AddressingMode::IMM, 2), // 0x49
    Instruction(BaseInstruction::LSR, AddressingMode::IMP, 2), // 0x4A
    Instruction(BaseInstruction::ALR, AddressingMode::IMM, 2), // 0x4B
    Instruction(BaseInstruction::JMP, AddressingMode::ABS, 3), // 0x4C
    Instruction(BaseInstruction::EOR, AddressingMode::ABS, 4), // 0x4D
    Instruction(BaseInstruction::LSR, AddressingMode::ABS, 6), // 0x4E
    Instruction(BaseInstruction::SRE, AddressingMode::ABS, 6), // 0x4F
    //
    Instruction(BaseInstruction::BVC, AddressingMode::REL, 2), // 0x50
    Instruction(BaseInstruction::EOR, AddressingMode::IZY, 5), // 0x51
    Instruction(BaseInstruction::HLT, AddressingMode::IMP, 0), // 0x52
    Instruction(BaseInstruction::SRE, AddressingMode::IZY, 8), // 0x53
    Instruction(BaseInstruction::NOP, AddressingMode::ZPX, 4), // 0x54
    Instruction(BaseInstruction::EOR, AddressingMode::ZPX, 4), // 0x55
    Instruction(BaseInstruction::LSR, AddressingMode::ZPX, 6), // 0x56
    Instruction(BaseInstruction::SRE, AddressingMode::ZPX, 6), // 0x57
    Instruction(BaseInstruction::CLI, AddressingMode::IMP, 2), // 0x58
    Instruction(BaseInstruction::EOR, AddressingMode::ABY, 4), // 0x59
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 2), // 0x5A
    Instruction(BaseInstruction::SRE, AddressingMode::ABY, 7), // 0x5B
    Instruction(BaseInstruction::NOP, AddressingMode::ABX, 4), // 0x5C
    Instruction(BaseInstruction::EOR, AddressingMode::ABX, 4), // 0x5D
    Instruction(BaseInstruction::LSR, AddressingMode::ABX, 7), // 0x5E
    Instruction(BaseInstruction::SRE, AddressingMode::ABX, 7), // 0x5F
    //
    Instruction(BaseInstruction::RTS, AddressingMode::IMP, 6), // 0x60
    Instruction(BaseInstruction::ADC, AddressingMode::IZX, 6), // 0x61
    Instruction(BaseInstruction::HLT, AddressingMode::IMP, 0), // 0x62
    Instruction(BaseInstruction::RRA, AddressingMode::IZX, 8), // 0x63
    Instruction(BaseInstruction::NOP, AddressingMode::ZP0, 3), // 0x64
    Instruction(BaseInstruction::ADC, AddressingMode::ZP0, 3), // 0x65
    Instruction(BaseInstruction::ROR, AddressingMode::ZP0, 5), // 0x66
    Instruction(BaseInstruction::RRA, AddressingMode::ZP0, 5), // 0x67
    Instruction(BaseInstruction::PLA, AddressingMode::IMP, 4), // 0x68
    Instruction(BaseInstruction::ADC, AddressingMode::IMM, 2), // 0x69
    Instruction(BaseInstruction::ROR, AddressingMode::IMP, 2), // 0x6A
    Instruction(BaseInstruction::ARR, AddressingMode::IMM, 2), // 0x6B
    Instruction(BaseInstruction::JMP, AddressingMode::IND, 5), // 0x6C
    Instruction(BaseInstruction::ADC, AddressingMode::ABS, 4), // 0x6D
    Instruction(BaseInstruction::ROR, AddressingMode::ABS, 6), // 0x6E
    Instruction(BaseInstruction::RRA, AddressingMode::ABS, 6), // 0x6F
    //
    Instruction(BaseInstruction::BVS, AddressingMode::REL, 2), // 0x70
    Instruction(BaseInstruction::ADC, AddressingMode::IZY, 5), // 0x71
    Instruction(BaseInstruction::HLT, AddressingMode::IMP, 0), // 0x72
    Instruction(BaseInstruction::RRA, AddressingMode::IZY, 8), // 0x73
    Instruction(BaseInstruction::NOP, AddressingMode::ZPX, 4), // 0x74
    Instruction(BaseInstruction::ADC, AddressingMode::ZPX, 4), // 0x75
    Instruction(BaseInstruction::ROR, AddressingMode::ZPX, 6), // 0x76
    Instruction(BaseInstruction::RRA, AddressingMode::ZPX, 6), // 0x77
    Instruction(BaseInstruction::SEI, AddressingMode::IMP, 2), // 0x78
    Instruction(BaseInstruction::ADC, AddressingMode::ABY, 4), // 0x79
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 2), // 0x7A
    Instruction(BaseInstruction::RRA, AddressingMode::ABY, 7), // 0x7B
    Instruction(BaseInstruction::NOP, AddressingMode::ABX, 4), // 0x7C
    Instruction(BaseInstruction::ADC, AddressingMode::ABX, 4), // 0x7D
    Instruction(BaseInstruction::ROR, AddressingMode::ABX, 7), // 0x7E
    Instruction(BaseInstruction::RRA, AddressingMode::ABX, 7), // 0x7F
    //
    Instruction(BaseInstruction::NOP, AddressingMode::IMM, 2), // 0x80
    Instruction(BaseInstruction::STA, AddressingMode::IZX, 6), // 0x81
    Instruction(BaseInstruction::NOP, AddressingMode::IMM, 2), // 0x82
    Instruction(BaseInstruction::SAX, AddressingMode::IZX, 6), // 0x83
    Instruction(BaseInstruction::STY, AddressingMode::ZP0, 3), // 0x84
    Instruction(BaseInstruction::STA, AddressingMode::ZP0, 3), // 0x85
    Instruction(BaseInstruction::STX, AddressingMode::ZP0, 3), // 0x86
    Instruction(BaseInstruction::SAX, AddressingMode::ZP0, 3), // 0x87
    Instruction(BaseInstruction::DEY, AddressingMode::IMP, 2), // 0x88
    Instruction(BaseInstruction::NOP, AddressingMode::IMM, 2), // 0x89
    Instruction(BaseInstruction::TXA, AddressingMode::IMP, 2), // 0x8A
    Instruction(BaseInstruction::XAA, AddressingMode::IMM, 2), // 0x8B
    Instruction(BaseInstruction::STY, AddressingMode::ABS, 4), // 0x8C
    Instruction(BaseInstruction::STA, AddressingMode::ABS, 4), // 0x8D
    Instruction(BaseInstruction::STX, AddressingMode::ABS, 4), // 0x8E
    Instruction(BaseInstruction::SAX, AddressingMode::ABS, 4), // 0x8F
    //
    Instruction(BaseInstruction::BCC, AddressingMode::REL, 2), // 0x90
    Instruction(BaseInstruction::STA, AddressingMode::IZY, 6), // 0x91
    Instruction(BaseInstruction::HLT, AddressingMode::IMP, 0), // 0x92
    Instruction(BaseInstruction::AHX, AddressingMode::IZY, 6), // 0x93
    Instruction(BaseInstruction::STY, AddressingMode::ZPX, 4), // 0x94
    Instruction(BaseInstruction::STA, AddressingMode::ZPX, 4), // 0x95
    Instruction(BaseInstruction::STX, AddressingMode::ZPY, 4), // 0x96
    Instruction(BaseInstruction::SAX, AddressingMode::ZPY, 4), // 0x97
    Instruction(BaseInstruction::TYA, AddressingMode::IMP, 2), // 0x98
    Instruction(BaseInstruction::STA, AddressingMode::ABY, 5), // 0x99
    Instruction(BaseInstruction::TXS, AddressingMode::IMP, 2), // 0x9A
    Instruction(BaseInstruction::TAS, AddressingMode::ABY, 5), // 0x9B
    Instruction(BaseInstruction::SHY, AddressingMode::ABX, 5), // 0x9C
    Instruction(BaseInstruction::STA, AddressingMode::ABX, 5), // 0x9D
    Instruction(BaseInstruction::SHX, AddressingMode::ABY, 5), // 0x9E
    Instruction(BaseInstruction::AHX, AddressingMode::ABY, 5), // 0x9F
    //
    Instruction(BaseInstruction::LDY, AddressingMode::IMM, 2), // 0xA0
    Instruction(BaseInstruction::LDA, AddressingMode::IZX, 6), // 0xA1
    Instruction(BaseInstruction::LDX, AddressingMode::IMM, 2), // 0xA2
    Instruction(BaseInstruction::LAX, AddressingMode::IZX, 6), // 0xA3
    Instruction(BaseInstruction::LDY, AddressingMode::ZP0, 3), // 0xA4
    Instruction(BaseInstruction::LDA, AddressingMode::ZP0, 3), // 0xA5
    Instruction(BaseInstruction::LDX, AddressingMode::ZP0, 3), // 0xA6
    Instruction(BaseInstruction::LAX, AddressingMode::ZP0, 3), // 0xA7
    Instruction(BaseInstruction::TAY, AddressingMode::IMP, 2), // 0xA8
    Instruction(BaseInstruction::LDA, AddressingMode::IMM, 2), // 0xA9
    Instruction(BaseInstruction::TAX, AddressingMode::IMP, 2), // 0xAA
    Instruction(BaseInstruction::LAX, AddressingMode::IMM, 2), // 0xAB
    Instruction(BaseInstruction::LDY, AddressingMode::ABS, 4), // 0xAC
    Instruction(BaseInstruction::LDA, AddressingMode::ABS, 4), // 0xAD
    Instruction(BaseInstruction::LDX, AddressingMode::ABS, 4), // 0xAE
    Instruction(BaseInstruction::LAX, AddressingMode::ABS, 4), // 0xAF
    //
    Instruction(BaseInstruction::BCS, AddressingMode::REL, 2), // 0xB0
    Instruction(BaseInstruction::LDA, AddressingMode::IZY, 5), // 0xB1
    Instruction(BaseInstruction::HLT, AddressingMode::IMP, 0), // 0xB2
    Instruction(BaseInstruction::LAX, AddressingMode::IZY, 5), // 0xB3
    Instruction(BaseInstruction::LDY, AddressingMode::ZPX, 4), // 0xB4
    Instruction(BaseInstruction::LDA, AddressingMode::ZPX, 4), // 0xB5
    Instruction(BaseInstruction::LDX, AddressingMode::ZPY, 4), // 0xB6
    Instruction(BaseInstruction::LAX, AddressingMode::ZPY, 4), // 0xB7
    Instruction(BaseInstruction::CLV, AddressingMode::IMP, 2), // 0xB8
    Instruction(BaseInstruction::LDA, AddressingMode::ABY, 4), // 0xB9
    Instruction(BaseInstruction::TSX, AddressingMode::IMP, 2), // 0xBA
    Instruction(BaseInstruction::LAS, AddressingMode::ABY, 4), // 0xBB
    Instruction(BaseInstruction::LDY, AddressingMode::ABX, 4), // 0xBC
    Instruction(BaseInstruction::LDA, AddressingMode::ABX, 4), // 0xBD
    Instruction(BaseInstruction::LDX, AddressingMode::ABY, 4), // 0xBE
    Instruction(BaseInstruction::LAX, AddressingMode::ABY, 4), // 0xBF
    //
    Instruction(BaseInstruction::CPY, AddressingMode::IMM, 2), // 0xC0
    Instruction(BaseInstruction::CMP, AddressingMode::IZX, 6), // 0xC1
    Instruction(BaseInstruction::NOP, AddressingMode::IMM, 2), // 0xC2
    Instruction(BaseInstruction::DCP, AddressingMode::IZX, 8), // 0xC3
    Instruction(BaseInstruction::CPY, AddressingMode::ZP0, 3), // 0xC4
    Instruction(BaseInstruction::CMP, AddressingMode::ZP0, 3), // 0xC5
    Instruction(BaseInstruction::DEC, AddressingMode::ZP0, 5), // 0xC6
    Instruction(BaseInstruction::DCP, AddressingMode::ZP0, 5), // 0xC7
    Instruction(BaseInstruction::INY, AddressingMode::IMP, 2), // 0xC8
    Instruction(BaseInstruction::CMP, AddressingMode::IMM, 2), // 0xC9
    Instruction(BaseInstruction::DEX, AddressingMode::IMP, 2), // 0xCA
    Instruction(BaseInstruction::AXS, AddressingMode::IMM, 2), // 0xCB
    Instruction(BaseInstruction::CPY, AddressingMode::ABS, 4), // 0xCC
    Instruction(BaseInstruction::CMP, AddressingMode::ABS, 4), // 0xCD
    Instruction(BaseInstruction::DEC, AddressingMode::ABS, 6), // 0xCE
    Instruction(BaseInstruction::DCP, AddressingMode::ABS, 6), // 0xCF
    //
    Instruction(BaseInstruction::BNE, AddressingMode::REL, 2), // 0xD0
    Instruction(BaseInstruction::CMP, AddressingMode::IZY, 5), // 0xD1
    Instruction(BaseInstruction::HLT, AddressingMode::IMP, 0), // 0xD2
    Instruction(BaseInstruction::DCP, AddressingMode::IZY, 8), // 0xD3
    Instruction(BaseInstruction::NOP, AddressingMode::ZPX, 4), // 0xD4
    Instruction(BaseInstruction::CMP, AddressingMode::ZPX, 4), // 0xD5
    Instruction(BaseInstruction::DEC, AddressingMode::ZPX, 6), // 0xD6
    Instruction(BaseInstruction::DCP, AddressingMode::ZPX, 6), // 0xD7
    Instruction(BaseInstruction::CLD, AddressingMode::IMP, 2), // 0xD8
    Instruction(BaseInstruction::CMP, AddressingMode::ABY, 4), // 0xD9
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 2), // 0xDA
    Instruction(BaseInstruction::DCP, AddressingMode::ABY, 7), // 0xDB
    Instruction(BaseInstruction::NOP, AddressingMode::ABX, 4), // 0xDC
    Instruction(BaseInstruction::CMP, AddressingMode::ABX, 4), // 0xDD
    Instruction(BaseInstruction::DEC, AddressingMode::ABX, 7), // 0xDE
    Instruction(BaseInstruction::DCP, AddressingMode::ABX, 7), // 0xDF
    //
    Instruction(BaseInstruction::CPX, AddressingMode::IMM, 2), // 0xE0
    Instruction(BaseInstruction::SBC, AddressingMode::IZX, 6), // 0xE1
    Instruction(BaseInstruction::NOP, AddressingMode::IMM, 2), // 0xE2
    Instruction(BaseInstruction::ISC, AddressingMode::IZX, 8), // 0xE3
    Instruction(BaseInstruction::CPX, AddressingMode::ZP0, 3), // 0xE4
    Instruction(BaseInstruction::SBC, AddressingMode::ZP0, 3), // 0xE5
    Instruction(BaseInstruction::INC, AddressingMode::ZP0, 5), // 0xE6
    Instruction(BaseInstruction::ISC, AddressingMode::ZP0, 5), // 0xE7
    Instruction(BaseInstruction::INX, AddressingMode::IMP, 2), // 0xE8
    Instruction(BaseInstruction::SBC, AddressingMode::IMM, 2), // 0xE9
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 2), // 0xEA
    Instruction(BaseInstruction::SBC, AddressingMode::IMM, 2), // 0xEB
    Instruction(BaseInstruction::CPX, AddressingMode::ABS, 4), // 0xEC
    Instruction(BaseInstruction::SBC, AddressingMode::ABS, 4), // 0xED
    Instruction(BaseInstruction::INC, AddressingMode::ABS, 6), // 0xEE
    Instruction(BaseInstruction::ISC, AddressingMode::ABS, 6), // 0xEF
    //
    Instruction(BaseInstruction::BEQ, AddressingMode::REL, 2), // 0xF0
    Instruction(BaseInstruction::SBC, AddressingMode::IZY, 5), // 0xF1
    Instruction(BaseInstruction::HLT, AddressingMode::IMP, 0), // 0xF2
    Instruction(BaseInstruction::ISC, AddressingMode::IZY, 8), // 0xF3
    Instruction(BaseInstruction::NOP, AddressingMode::ZPX, 4), // 0xF4
    Instruction(BaseInstruction::SBC, AddressingMode::ZPX, 4), // 0xF5
    Instruction(BaseInstruction::INC, AddressingMode::ZPX, 6), // 0xF6
    Instruction(BaseInstruction::ISC, AddressingMode::ZPX, 6), // 0xF7
    Instruction(BaseInstruction::SED, AddressingMode::IMP, 2), // 0xF8
    Instruction(BaseInstruction::SBC, AddressingMode::ABY, 4), // 0xF9
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 2), // 0xFA
    Instruction(BaseInstruction::ISC, AddressingMode::ABY, 7), // 0xFB
    Instruction(BaseInstruction::NOP, AddressingMode::ABX, 4), // 0xFC
    Instruction(BaseInstruction::SBC, AddressingMode::ABX, 4), // 0xFD
    Instruction(BaseInstruction::INC, AddressingMode::ABX, 7), // 0xFE
    Instruction(BaseInstruction::ISC, AddressingMode::ABX, 7), // 0xFF
];