use crate::bus::Bus;
use crate::cpu::*;
use crate::types::*;
use strum_macros::{AsRefStr, IntoStaticStr};

pub type Address = u16w;
pub type Word = u8w;

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

#[derive(PartialEq, Eq, Clone, Copy, Debug, strum_macros::Display, AsRefStr, IntoStaticStr)]
enum AddressingMode {
    /// Implied
    IMP,
    /// Immediate
    IMM,
    /// Zero-page
    ZP0,
    /// Zero-page + relative offset
    ZPR,
    /// Zero-page + X register offset
    ZPX,
    /// Zero-page + Y register offset
    ZPY,
    /// Relative
    REL,
    /// Absolute
    ABS,
    /// Absolute + X register offset
    ABX,
    /// Absolute + Y register offset
    ABY,
    /// Indirect
    IND,
    /// Indirect zero page
    IZP,
    /// Indirect (zero-page + X register offset)
    IZX,
    /// (Indirect zero-page) + Y register offset
    IZY,
    /// Indirect + X register offset
    IAX,
}
impl AddressingMode {
    fn read_next(&self, cpu: &mut Cpu6502) -> InstructionData {
        match self {
            AddressingMode::IMP => InstructionData::IMP,
            AddressingMode::IMM => InstructionData::IMM(cpu.read_next_word()),
            AddressingMode::ZP0 => InstructionData::ZP0(cpu.read_next_word()),
            AddressingMode::ZPR => InstructionData::ZPR(cpu.read_next_word(), cpu.read_next_word()),
            AddressingMode::ZPX => InstructionData::ZPX(cpu.read_next_word()),
            AddressingMode::ZPY => InstructionData::ZPY(cpu.read_next_word()),
            AddressingMode::REL => InstructionData::REL(cpu.read_next_word()),
            AddressingMode::ABS => InstructionData::ABS(cpu.read_next_address()),
            AddressingMode::ABX => InstructionData::ABX(cpu.read_next_address()),
            AddressingMode::ABY => InstructionData::ABY(cpu.read_next_address()),
            AddressingMode::IND => InstructionData::IND(cpu.read_next_address()),
            AddressingMode::IZP => InstructionData::IZP(cpu.read_next_word()),
            AddressingMode::IZX => InstructionData::IZX(cpu.read_next_word()),
            AddressingMode::IZY => InstructionData::IZY(cpu.read_next_word()),
            AddressingMode::IAX => InstructionData::IAX(cpu.read_next_address()),
        }
    }

    fn read(&self, cpu: &Cpu6502, address: Address) -> InstructionData {
        match self {
            AddressingMode::IMP => InstructionData::IMP,
            AddressingMode::IMM => InstructionData::IMM(cpu.read_word(address)),
            AddressingMode::ZP0 => InstructionData::ZP0(cpu.read_word(address)),
            AddressingMode::ZPR => {
                InstructionData::ZPR(cpu.read_word(address), cpu.read_word(address + Wrapping(1)))
            }
            AddressingMode::ZPX => InstructionData::ZPX(cpu.read_word(address)),
            AddressingMode::ZPY => InstructionData::ZPY(cpu.read_word(address)),
            AddressingMode::REL => InstructionData::REL(cpu.read_word(address)),
            AddressingMode::ABS => InstructionData::ABS(cpu.read_address(address)),
            AddressingMode::ABX => InstructionData::ABX(cpu.read_address(address)),
            AddressingMode::ABY => InstructionData::ABY(cpu.read_address(address)),
            AddressingMode::IND => InstructionData::IND(cpu.read_address(address)),
            AddressingMode::IZP => InstructionData::IZP(cpu.read_word(address)),
            AddressingMode::IZX => InstructionData::IZX(cpu.read_word(address)),
            AddressingMode::IZY => InstructionData::IZY(cpu.read_word(address)),
            AddressingMode::IAX => InstructionData::IAX(cpu.read_address(address)),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, strum_macros::Display, AsRefStr, IntoStaticStr)]
enum BaseInstruction {
    LDA,
    LDX,
    LDY,
    STA,
    STX,
    STY,
    TAX,
    TAY,
    TXA,
    TYA,
    TSX,
    TXS,
    PHA,
    PHP,
    PLA,
    PLP,
    AND,
    EOR,
    ORA,
    BIT,
    ADC,
    SBC,
    CMP,
    CPX,
    CPY,
    INC,
    INX,
    INY,
    DEC,
    DEX,
    DEY,
    ASL,
    LSR,
    ROL,
    ROR,
    JMP,
    JSR,
    RTS,
    BCC,
    BCS,
    BEQ,
    BMI,
    BNE,
    BPL,
    BVC,
    BVS,
    CLC,
    CLD,
    CLI,
    CLV,
    SEC,
    SED,
    SEI,
    BRK,
    NOP,
    RTI,

    // Undocumented instructions
    SLO,
    ANC,
    RLA,
    SRE,
    ALR,
    RRA,
    ARR,
    SAX,
    XAA,
    AHX,
    TAS,
    SHY,
    SHX,
    LAX,
    LAS,
    DCP,
    AXS,
    ISC,
    HLT,

    // 65C02 instructions
    BRA,
    PHX,
    PHY,
    PLX,
    PLY,
    STZ,
    TRB,
    TSB,
    BBR0,
    BBR1,
    BBR2,
    BBR3,
    BBR4,
    BBR5,
    BBR6,
    BBR7,
    BBS0,
    BBS1,
    BBS2,
    BBS3,
    BBS4,
    BBS5,
    BBS6,
    BBS7,
    RMB0,
    RMB1,
    RMB2,
    RMB3,
    RMB4,
    RMB5,
    RMB6,
    RMB7,
    SMB0,
    SMB1,
    SMB2,
    SMB3,
    SMB4,
    SMB5,
    SMB6,
    SMB7,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
struct Instruction(BaseInstruction, AddressingMode, u32, bool);

#[derive(Debug)]
enum ExecutionData {
    None,
    Data(Word),
    Address(Address),
    AddressPair(Address, Address),
}
impl ExecutionData {
    fn read_data(&self, cpu: &Cpu6502) -> Word {
        match self {
            Self::Data(data) => *data,
            Self::Address(address) => cpu.read_word(*address),
            Self::AddressPair(address, _) => cpu.read_word(*address),
            _ => panic!("Invalid addressing mode"),
        }
    }

    fn write_data(&self, cpu: &Cpu6502, data: Word) {
        match self {
            Self::Address(address) => cpu.write_word(*address, data),
            Self::AddressPair(address, _) => cpu.write_word(*address, data),
            _ => panic!("Invalid addressing mode"),
        };
    }

    fn read_address(&self) -> Address {
        match self {
            Self::Address(address) => *address,
            Self::AddressPair(_, address) => *address,
            _ => panic!("Invalid addressing mode"),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum InstructionData {
    IMP,
    IMM(Word),
    ZP0(Word),
    ZPR(Word, Word),
    ZPX(Word),
    ZPY(Word),
    REL(Word),
    ABS(Address),
    ABX(Address),
    ABY(Address),
    IND(Address),
    IZP(Word),
    IZX(Word),
    IZY(Word),
    IAX(Address),
}
impl InstructionData {
    fn to_execution_data(&self, cpu: &Cpu6502) -> (ExecutionData, bool) {
        fn rel_to_abs(cpu: &Cpu6502, rel_address: Word) -> (Address, bool) {
            let mut address = rel_address.0 as u16;
            // Handle the negative case
            if (address & 0x0080) != 0 {
                address |= 0xFF00;
            }

            let abs_address = cpu.pc + Wrapping(address);
            let page_before = cpu.pc & Wrapping(0xFF00);
            let page_after = abs_address & Wrapping(0xFF00);
            let page_crossed = page_before != page_after;

            (abs_address, page_crossed)
        }

        match self {
            InstructionData::IMP => (ExecutionData::None, false),
            InstructionData::IMM(data) => (ExecutionData::Data(*data), false),
            InstructionData::ZP0(zp_address) => {
                (ExecutionData::Address(Wrapping(zp_address.0 as u16)), false)
            }
            InstructionData::ZPR(zp_address, rel_address) => {
                let (abs_address, page_crossed) = rel_to_abs(cpu, *rel_address);
                (
                    ExecutionData::AddressPair(Wrapping(zp_address.0 as u16), abs_address),
                    page_crossed,
                )
            }
            InstructionData::ZPX(zp_address) => (
                ExecutionData::Address(Wrapping((zp_address + cpu.x).0 as u16)),
                false,
            ),
            InstructionData::ZPY(zp_address) => (
                ExecutionData::Address(Wrapping((zp_address + cpu.y).0 as u16)),
                false,
            ),
            InstructionData::REL(rel_address) => {
                let (abs_address, page_crossed) = rel_to_abs(cpu, *rel_address);
                (ExecutionData::Address(abs_address), page_crossed)
            }
            InstructionData::ABS(abs_address) => (ExecutionData::Address(*abs_address), false),
            InstructionData::ABX(abs_address) => {
                let address_after = abs_address + Wrapping(cpu.x.0 as u16);
                let page_before = abs_address & Wrapping(0xFF00);
                let page_after = address_after & Wrapping(0xFF00);

                let page_crossed = page_before != page_after;
                (ExecutionData::Address(address_after), page_crossed)
            }
            InstructionData::ABY(abs_address) => {
                let address_after = abs_address + Wrapping(cpu.y.0 as u16);
                let page_before = abs_address & Wrapping(0xFF00);
                let page_after = address_after & Wrapping(0xFF00);

                let page_crossed = page_before != page_after;
                (ExecutionData::Address(address_after), page_crossed)
            }
            InstructionData::IND(ind_address) => {
                let address = cpu.read_address_ind(*ind_address);
                (ExecutionData::Address(address), false)
            }
            InstructionData::IZP(ind_address) => {
                let address = cpu.read_address_ind(Wrapping(ind_address.0 as u16));
                (ExecutionData::Address(address), false)
            }
            InstructionData::IZX(ind_address) => {
                let address = cpu.read_address_ind(Wrapping((ind_address + cpu.x).0 as u16));
                (ExecutionData::Address(address), false)
            }
            InstructionData::IZY(ind_address) => {
                let address_before = cpu.read_address_ind(Wrapping(ind_address.0 as u16));
                let page_before = address_before & Wrapping(0xFF00);

                let address_after = address_before + Wrapping(cpu.y.0 as u16);
                let page_after = address_after & Wrapping(0xFF00);

                let page_crossed = page_before != page_after;
                (ExecutionData::Address(address_after), page_crossed)
            }
            InstructionData::IAX(ind_address) => {
                let address = cpu.read_address_ind(ind_address + Wrapping(cpu.x.0 as u16));
                (ExecutionData::Address(address), false)
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Asm6502Instruction {
    is_undefined: bool,
    address: Address,
    instruction: BaseInstruction,
    data: InstructionData,
}
impl Asm6502Instruction {
    const UNDEFINED: Self = Self {
        is_undefined: true,
        address: Wrapping(0),
        instruction: BaseInstruction::HLT,
        data: InstructionData::IMP,
    };

    #[inline]
    const fn new(address: Address, instruction: BaseInstruction, data: InstructionData) -> Self {
        Self {
            is_undefined: false,
            address,
            instruction,
            data,
        }
    }
}
impl Display for Asm6502Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_undefined {
            f.write_fmt(format_args!("UNKNOWN"))
        } else {
            match self.data {
                InstructionData::IMP => f.write_str(self.instruction.into()),
                InstructionData::IMM(data) => {
                    f.write_fmt(format_args!("{:<4} #${:0>2X}", self.instruction, data))
                }
                InstructionData::ZP0(zp_address) => {
                    f.write_fmt(format_args!("{:<4} ${:0>2X}", self.instruction, zp_address))
                }
                InstructionData::ZPR(zp_address, rel_address) => f.write_fmt(format_args!(
                    "{:<4} ${:0>2X},${:0>2X}",
                    self.instruction, zp_address, rel_address
                )),
                InstructionData::ZPX(zp_address) => f.write_fmt(format_args!(
                    "{:<4} ${:0>2X},X",
                    self.instruction, zp_address
                )),
                InstructionData::ZPY(zp_address) => f.write_fmt(format_args!(
                    "{:<4} ${:0>2X},Y",
                    self.instruction, zp_address
                )),
                InstructionData::REL(rel_address) => f.write_fmt(format_args!(
                    "{:<4} ${:0>2X}",
                    self.instruction, rel_address
                )),
                InstructionData::ABS(abs_address) => f.write_fmt(format_args!(
                    "{:<4} ${:0>4X}",
                    self.instruction, abs_address
                )),
                InstructionData::ABX(abs_address) => f.write_fmt(format_args!(
                    "{:<4} ${:0>4X},X",
                    self.instruction, abs_address
                )),
                InstructionData::ABY(abs_address) => f.write_fmt(format_args!(
                    "{:<4} ${:0>4X},Y",
                    self.instruction, abs_address
                )),
                InstructionData::IND(ind_address) => f.write_fmt(format_args!(
                    "{:<4} (${:0>4X})",
                    self.instruction, ind_address
                )),
                InstructionData::IZP(ind_address) => f.write_fmt(format_args!(
                    "{:<4} (${:0>2X})",
                    self.instruction, ind_address
                )),
                InstructionData::IZX(ind_address) => f.write_fmt(format_args!(
                    "{:<4} (${:0>2X},X)",
                    self.instruction, ind_address
                )),
                InstructionData::IZY(ind_address) => f.write_fmt(format_args!(
                    "{:<4} (${:0>2X}),Y",
                    self.instruction, ind_address
                )),
                InstructionData::IAX(ind_address) => f.write_fmt(format_args!(
                    "{:<4} (${:0>4X},X)",
                    self.instruction, ind_address
                )),
            }
        }
    }
}
impl AsmInstruction<Address> for Asm6502Instruction {
    #[inline]
    fn address(&self) -> Address {
        self.address
    }

    fn byte_size(&self) -> usize {
        match self.data {
            InstructionData::IMP => 1,
            InstructionData::IMM(_) => 2,
            InstructionData::ZP0(_) => 2,
            InstructionData::ZPR(_, _) => 3,
            InstructionData::ZPX(_) => 2,
            InstructionData::ZPY(_) => 2,
            InstructionData::REL(_) => 2,
            InstructionData::ABS(_) => 3,
            InstructionData::ABX(_) => 3,
            InstructionData::ABY(_) => 3,
            InstructionData::IND(_) => 3,
            InstructionData::IZP(_) => 2,
            InstructionData::IZX(_) => 2,
            InstructionData::IZY(_) => 2,
            InstructionData::IAX(_) => 3,
        }
    }

    #[inline]
    fn mnemonic(&self) -> &str {
        self.instruction.into()
    }
}

const STACK_BASE: Address = Wrapping(0x0100); // Stack base address
const IRQ_VECTOR: Address = Wrapping(0xFFFE); // Where to load the program counter from when an interrupt occurs
const NMI_VECTOR: Address = Wrapping(0xFFFA); // Where to load the program counter from when a non-maskable interrupt occurs
const RESET_VECTOR: Address = Wrapping(0xFFFC); // Where to load the program counter from when a reset occurs
const SP_INIT: Word = Wrapping(0xFD); // The initial top of the stack

pub struct Cpu6502<'a> {
    /// Accumulator
    a: Word,
    /// X index register
    x: Word,
    /// Y index register
    y: Word,
    /// Stack pointer
    sp: Word,
    /// Program counter
    pc: Address,
    /// Status register
    status: StatusFlags,

    bus: EmuRef<Bus<'a, Address, Word>>,
    emulate_indirect_jmp_bug: bool,
    emulate_invalid_decimal_flags: bool,
    enable_decimal_mode: bool,
}
impl<'a> Cpu6502<'a> {
    pub const fn new(bus: EmuRef<Bus<'a, Address, Word>>, enable_decimal_mode: bool) -> Self {
        Self {
            a: Wrapping(0),
            x: Wrapping(0),
            y: Wrapping(0),
            sp: Wrapping(0),
            pc: Wrapping(0),
            status: StatusFlags::empty(),
            bus,
            emulate_indirect_jmp_bug: true,
            emulate_invalid_decimal_flags: true,
            enable_decimal_mode,
        }
    }

    #[inline]
    pub fn create(bus: EmuRef<Bus<'a, Address, Word>>, enable_decimal_mode: bool) -> EmuRef<Self> {
        make_ref(Self::new(bus, enable_decimal_mode))
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
        if self.emulate_indirect_jmp_bug {
            let bus_borrow = self.bus.borrow();

            // Bug in the original hardware
            let page = address & Wrapping(0xFF00);
            let hi_address = ((address + Wrapping(1)) & Wrapping(0x00FF)) | page;

            let lo = bus_borrow.read(address);
            let hi = bus_borrow.read(hi_address);
            Wrapping((lo.0 as u16) | ((hi.0 as u16) << 8))
        } else {
            self.read_address(address)
        }
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
        INSTRUCTION_LOOKUP_6502[op_code]
    }

    fn execute_instruction(&mut self, instruction: Instruction) -> u32 {
        let base_instruction = instruction.0;
        let addressing_mode = instruction.1;
        let cycles = instruction.2;
        let add_cycle_on_page_cross = instruction.3;

        let instruction_data = addressing_mode.read_next(self);
        let (execution_data, page_crossed) = instruction_data.to_execution_data(self);

        let additional_cycles = match base_instruction {
            BaseInstruction::LDA => self.execute_lda(execution_data),
            BaseInstruction::LDX => self.execute_ldx(execution_data),
            BaseInstruction::LDY => self.execute_ldy(execution_data),
            BaseInstruction::STA => self.execute_sta(execution_data),
            BaseInstruction::STX => self.execute_stx(execution_data),
            BaseInstruction::STY => self.execute_sty(execution_data),
            BaseInstruction::TAX => self.execute_tax(),
            BaseInstruction::TAY => self.execute_tay(),
            BaseInstruction::TXA => self.execute_txa(),
            BaseInstruction::TYA => self.execute_tya(),
            BaseInstruction::TSX => self.execute_tsx(),
            BaseInstruction::TXS => self.execute_txs(),
            BaseInstruction::PHA => self.execute_pha(),
            BaseInstruction::PHP => self.execute_php(),
            BaseInstruction::PLA => self.execute_pla(),
            BaseInstruction::PLP => self.execute_plp(),
            BaseInstruction::AND => self.execute_and(execution_data),
            BaseInstruction::EOR => self.execute_eor(execution_data),
            BaseInstruction::ORA => self.execute_ora(execution_data),
            BaseInstruction::BIT => self.execute_bit(execution_data),
            BaseInstruction::ADC => self.execute_adc(execution_data),
            BaseInstruction::SBC => self.execute_sbc(execution_data),
            BaseInstruction::CMP => self.execute_cmp(execution_data),
            BaseInstruction::CPX => self.execute_cpx(execution_data),
            BaseInstruction::CPY => self.execute_cpy(execution_data),
            BaseInstruction::INC => self.execute_inc(execution_data),
            BaseInstruction::INX => self.execute_inx(),
            BaseInstruction::INY => self.execute_iny(),
            BaseInstruction::DEC => self.execute_dec(execution_data),
            BaseInstruction::DEX => self.execute_dex(),
            BaseInstruction::DEY => self.execute_dey(),
            BaseInstruction::ASL => self.execute_asl(execution_data),
            BaseInstruction::LSR => self.execute_lsr(execution_data),
            BaseInstruction::ROL => self.execute_rol(execution_data),
            BaseInstruction::ROR => self.execute_ror(execution_data),
            BaseInstruction::JMP => self.execute_jmp(execution_data),
            BaseInstruction::JSR => self.execute_jsr(execution_data),
            BaseInstruction::RTS => self.execute_rts(execution_data),
            BaseInstruction::BCC => self.execute_bcc(execution_data),
            BaseInstruction::BCS => self.execute_bcs(execution_data),
            BaseInstruction::BEQ => self.execute_beq(execution_data),
            BaseInstruction::BMI => self.execute_bmi(execution_data),
            BaseInstruction::BNE => self.execute_bne(execution_data),
            BaseInstruction::BPL => self.execute_bpl(execution_data),
            BaseInstruction::BVC => self.execute_bvc(execution_data),
            BaseInstruction::BVS => self.execute_bvs(execution_data),
            BaseInstruction::CLC => self.execute_clc(),
            BaseInstruction::CLD => self.execute_cld(),
            BaseInstruction::CLI => self.execute_cli(),
            BaseInstruction::CLV => self.execute_clv(),
            BaseInstruction::SEC => self.execute_sec(),
            BaseInstruction::SED => self.execute_sed(),
            BaseInstruction::SEI => self.execute_sei(),
            BaseInstruction::BRK => self.execute_brk(),
            BaseInstruction::NOP => 0,
            BaseInstruction::RTI => self.execute_rti(),
            BaseInstruction::SLO => self.execute_slo(execution_data),
            BaseInstruction::ANC => self.execute_anc(execution_data),
            BaseInstruction::RLA => self.execute_rla(execution_data),
            BaseInstruction::SRE => self.execute_sre(execution_data),
            BaseInstruction::ALR => self.execute_alr(execution_data),
            BaseInstruction::RRA => self.execute_rra(execution_data),
            BaseInstruction::ARR => self.execute_arr(execution_data),
            BaseInstruction::SAX => self.execute_sax(execution_data),
            BaseInstruction::XAA => self.execute_xaa(execution_data),
            BaseInstruction::AHX => self.execute_ahx(execution_data),
            BaseInstruction::TAS => self.execute_tas(execution_data),
            BaseInstruction::SHY => self.execute_shy(execution_data),
            BaseInstruction::SHX => self.execute_shx(execution_data),
            BaseInstruction::LAX => self.execute_lax(execution_data),
            BaseInstruction::LAS => self.execute_las(execution_data),
            BaseInstruction::DCP => self.execute_dcp(execution_data),
            BaseInstruction::AXS => self.execute_axs(execution_data),
            BaseInstruction::ISC => self.execute_isc(execution_data),
            BaseInstruction::HLT => panic!("Invalid instruction"),
            BaseInstruction::BRA => self.execute_bra(execution_data),
            BaseInstruction::PHX => self.execute_phx(),
            BaseInstruction::PHY => self.execute_phy(),
            BaseInstruction::PLX => self.execute_plx(),
            BaseInstruction::PLY => self.execute_ply(),
            BaseInstruction::STZ => self.execute_stz(execution_data),
            BaseInstruction::TRB => self.execute_trb(execution_data),
            BaseInstruction::TSB => self.execute_tsb(execution_data),
            BaseInstruction::BBR0 => self.execute_bbr(execution_data, 0),
            BaseInstruction::BBR1 => self.execute_bbr(execution_data, 1),
            BaseInstruction::BBR2 => self.execute_bbr(execution_data, 2),
            BaseInstruction::BBR3 => self.execute_bbr(execution_data, 3),
            BaseInstruction::BBR4 => self.execute_bbr(execution_data, 4),
            BaseInstruction::BBR5 => self.execute_bbr(execution_data, 5),
            BaseInstruction::BBR6 => self.execute_bbr(execution_data, 6),
            BaseInstruction::BBR7 => self.execute_bbr(execution_data, 7),
            BaseInstruction::BBS0 => self.execute_bbs(execution_data, 0),
            BaseInstruction::BBS1 => self.execute_bbs(execution_data, 1),
            BaseInstruction::BBS2 => self.execute_bbs(execution_data, 2),
            BaseInstruction::BBS3 => self.execute_bbs(execution_data, 3),
            BaseInstruction::BBS4 => self.execute_bbs(execution_data, 4),
            BaseInstruction::BBS5 => self.execute_bbs(execution_data, 5),
            BaseInstruction::BBS6 => self.execute_bbs(execution_data, 6),
            BaseInstruction::BBS7 => self.execute_bbs(execution_data, 7),
            BaseInstruction::RMB0 => self.execute_rmb(execution_data, 0),
            BaseInstruction::RMB1 => self.execute_rmb(execution_data, 1),
            BaseInstruction::RMB2 => self.execute_rmb(execution_data, 2),
            BaseInstruction::RMB3 => self.execute_rmb(execution_data, 3),
            BaseInstruction::RMB4 => self.execute_rmb(execution_data, 4),
            BaseInstruction::RMB5 => self.execute_rmb(execution_data, 5),
            BaseInstruction::RMB6 => self.execute_rmb(execution_data, 6),
            BaseInstruction::RMB7 => self.execute_rmb(execution_data, 7),
            BaseInstruction::SMB0 => self.execute_smb(execution_data, 0),
            BaseInstruction::SMB1 => self.execute_smb(execution_data, 1),
            BaseInstruction::SMB2 => self.execute_smb(execution_data, 2),
            BaseInstruction::SMB3 => self.execute_smb(execution_data, 3),
            BaseInstruction::SMB4 => self.execute_smb(execution_data, 4),
            BaseInstruction::SMB5 => self.execute_smb(execution_data, 5),
            BaseInstruction::SMB6 => self.execute_smb(execution_data, 6),
            BaseInstruction::SMB7 => self.execute_smb(execution_data, 7),
        };

        cycles
            + if page_crossed && add_cycle_on_page_cross {
                1
            } else {
                0
            }
            + additional_cycles
    }

    fn disassemble(&self, address: Address, lookup: &[Instruction; 256]) -> Asm6502Instruction {
        let op_code = self.read_word(address).0 as usize;
        let instruction = lookup[op_code];
        let base_instruction = instruction.0;
        let addressing_mode = instruction.1;

        let instruction_data = addressing_mode.read(self, address + Wrapping(1));
        Asm6502Instruction::new(address, base_instruction, instruction_data)
    }

    fn disassemble_forward(
        &self,
        mut address: Address,
        n: usize,
        lookup: &[Instruction; 256],
    ) -> Box<[Asm6502Instruction]> {
        let mut instructions: Vec<Asm6502Instruction> = Vec::with_capacity(n);
        for _ in 0..n {
            let instruction = self.disassemble(address, lookup);
            instructions.push(instruction);
            address += Wrapping(instruction.byte_size() as u16);
        }
        instructions.into_boxed_slice()
    }

    fn disassemble_backward(
        &self,
        address: Address,
        n: usize,
        lookup: &[Instruction; 256],
    ) -> Box<[Asm6502Instruction]> {
        // This does not necessarily find the actual disassembly, only a good guess

        fn disassemble_up_to(
            cpu: &Cpu6502,
            mut address: Address,
            end: Address,
            lookup: &[Instruction; 256],
        ) -> (Address, Box<[Asm6502Instruction]>) {
            let mut instructions: Vec<Asm6502Instruction> = Vec::new();
            while address < end {
                let instruction = cpu.disassemble(address, lookup);
                instructions.push(instruction);
                address += Wrapping(instruction.byte_size() as u16);
            }
            (address - end, instructions.into_boxed_slice())
        }

        fn search_disassemblies(
            cpu: &Cpu6502,
            address: Address,
            n: usize,
            lookup: &[Instruction; 256],
        ) -> Option<Box<[Asm6502Instruction]>> {
            let mut search_address = address - Wrapping((n as u16) * 3);
            while search_address != address {
                let (overshoot, search_result) =
                    disassemble_up_to(cpu, search_address, address, lookup);
                if overshoot.0 == 0 {
                    // Search address yielded a dissasembly of correct size
                    return Some(search_result);
                } else {
                    search_address += Wrapping(1);
                }
            }
            None
        }

        let mut instructions = vec![Asm6502Instruction::UNDEFINED; n];
        if let Some(search_result) = search_disassemblies(self, address, n, lookup) {
            let result_start = n.saturating_sub(search_result.len());
            let result_offset = search_result.len().saturating_sub(n);
            instructions[result_start..].copy_from_slice(&search_result[result_offset..]);
        }

        instructions.into_boxed_slice()
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
impl<'a> Display for Cpu6502<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("N  V  -  B  D  I  Z  C\n{}  {}  {}  {}  {}  {}  {}  {}\nA: ${:0>2X}  X: ${:0>2X}  Y: ${:0>2X}\nPC: ${:0>4X}    SP: $01{:0>2X}",
        self.status.contains(StatusFlags::N) as u8,
        self.status.contains(StatusFlags::V) as u8,
        self.status.contains(StatusFlags::U) as u8,
        self.status.contains(StatusFlags::B) as u8,
        self.status.contains(StatusFlags::D) as u8,
        self.status.contains(StatusFlags::I) as u8,
        self.status.contains(StatusFlags::Z) as u8,
        self.status.contains(StatusFlags::C) as u8,
        self.a, self.x, self.y, self.pc, self.sp))
    }
}
impl<'a> Cpu<Address, Word, Asm6502Instruction> for Cpu6502<'a> {
    fn reset(&mut self) -> u32 {
        self.a = Wrapping(0);
        self.x = Wrapping(0);
        self.y = Wrapping(0);
        self.sp = SP_INIT;
        self.status = StatusFlags::U;
        self.pc = self.read_address(RESET_VECTOR);

        8
    }

    #[inline]
    fn execute_next_instruction(&mut self) -> u32 {
        let instruction = self.read_next_instruction();
        self.execute_instruction(instruction)
    }

    fn disassemble_current(&self, range: usize) -> Box<[Asm6502Instruction]> {
        let back = self.disassemble_backward(self.pc, range, &INSTRUCTION_LOOKUP_6502);
        let front = self.disassemble_forward(self.pc, range + 1, &INSTRUCTION_LOOKUP_6502);

        let mut result = vec![Asm6502Instruction::UNDEFINED; back.len() + front.len()];
        result[..back.len()].copy_from_slice(&back);
        result[back.len()..].copy_from_slice(&front);
        result.into_boxed_slice()
    }
}

pub struct Cpu65C02<'a> {
    base_cpu: Cpu6502<'a>,
}
impl<'a> Cpu65C02<'a> {
    #[inline]
    pub const fn new(bus: EmuRef<Bus<'a, Address, Word>>, enable_decimal_mode: bool) -> Self {
        let mut base_cpu = Cpu6502::new(bus, enable_decimal_mode);
        base_cpu.emulate_indirect_jmp_bug = false; // Fixed
        base_cpu.emulate_invalid_decimal_flags = false;
        Self { base_cpu }
    }

    #[inline]
    pub fn create(bus: EmuRef<Bus<'a, Address, Word>>, enable_decimal_mode: bool) -> EmuRef<Self> {
        make_ref(Self::new(bus, enable_decimal_mode))
    }

    #[inline]
    fn read_next_instruction(&mut self) -> Instruction {
        let op_code = self.base_cpu.read_next_word().0 as usize;
        INSTRUCTION_LOOKUP_65C02[op_code]
    }

    #[inline]
    pub fn irq(&mut self) -> u32 {
        self.base_cpu.irq()
    }

    #[inline]
    pub fn nmi(&mut self) -> u32 {
        self.base_cpu.nmi()
    }
}
impl<'a> Display for Cpu65C02<'a> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.base_cpu.fmt(f)
    }
}
impl<'a> Cpu<Address, Word, Asm6502Instruction> for Cpu65C02<'a> {
    #[inline]
    fn reset(&mut self) -> u32 {
        self.base_cpu.reset()
    }

    #[inline]
    fn execute_next_instruction(&mut self) -> u32 {
        let instruction = self.read_next_instruction();
        self.base_cpu.execute_instruction(instruction)
    }

    fn disassemble_current(&self, range: usize) -> Box<[Asm6502Instruction]> {
        let back =
            self.base_cpu
                .disassemble_backward(self.base_cpu.pc, range, &INSTRUCTION_LOOKUP_65C02);
        let front = self.base_cpu.disassemble_forward(
            self.base_cpu.pc,
            range + 1,
            &INSTRUCTION_LOOKUP_65C02,
        );

        let mut result = vec![Asm6502Instruction::UNDEFINED; back.len() + front.len()];
        result[..back.len()].copy_from_slice(&back);
        result[back.len()..].copy_from_slice(&front);
        result.into_boxed_slice()
    }
}

const INSTRUCTION_LOOKUP_6502: [Instruction; 256] = [
    Instruction(BaseInstruction::BRK, AddressingMode::IMP, 7, false), // 0x00
    Instruction(BaseInstruction::ORA, AddressingMode::IZX, 6, false), // 0x01
    Instruction(BaseInstruction::HLT, AddressingMode::IMP, 0, false), // 0x02
    Instruction(BaseInstruction::SLO, AddressingMode::IZX, 8, false), // 0x03
    Instruction(BaseInstruction::NOP, AddressingMode::ZP0, 3, false), // 0x04
    Instruction(BaseInstruction::ORA, AddressingMode::ZP0, 3, false), // 0x05
    Instruction(BaseInstruction::ASL, AddressingMode::ZP0, 5, false), // 0x06
    Instruction(BaseInstruction::SLO, AddressingMode::ZP0, 5, false), // 0x07
    Instruction(BaseInstruction::PHP, AddressingMode::IMP, 3, false), // 0x08
    Instruction(BaseInstruction::ORA, AddressingMode::IMM, 2, false), // 0x09
    Instruction(BaseInstruction::ASL, AddressingMode::IMP, 2, false), // 0x0A
    Instruction(BaseInstruction::ANC, AddressingMode::IMM, 2, false), // 0x0B
    Instruction(BaseInstruction::NOP, AddressingMode::ABS, 4, false), // 0x0C
    Instruction(BaseInstruction::ORA, AddressingMode::ABS, 4, false), // 0x0D
    Instruction(BaseInstruction::ASL, AddressingMode::ABS, 6, false), // 0x0E
    Instruction(BaseInstruction::SLO, AddressingMode::ABS, 6, false), // 0x0F
    //
    Instruction(BaseInstruction::BPL, AddressingMode::REL, 2, true), // 0x10
    Instruction(BaseInstruction::ORA, AddressingMode::IZY, 5, true), // 0x11
    Instruction(BaseInstruction::HLT, AddressingMode::IMP, 0, false), // 0x12
    Instruction(BaseInstruction::SLO, AddressingMode::IZY, 8, false), // 0x13
    Instruction(BaseInstruction::NOP, AddressingMode::ZPX, 4, false), // 0x14
    Instruction(BaseInstruction::ORA, AddressingMode::ZPX, 4, false), // 0x15
    Instruction(BaseInstruction::ASL, AddressingMode::ZPX, 6, false), // 0x16
    Instruction(BaseInstruction::SLO, AddressingMode::ZPX, 6, false), // 0x17
    Instruction(BaseInstruction::CLC, AddressingMode::IMP, 2, false), // 0x18
    Instruction(BaseInstruction::ORA, AddressingMode::ABY, 4, true), // 0x19
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 2, false), // 0x1A
    Instruction(BaseInstruction::SLO, AddressingMode::ABY, 7, false), // 0x1B
    Instruction(BaseInstruction::NOP, AddressingMode::ABX, 4, true), // 0x1C
    Instruction(BaseInstruction::ORA, AddressingMode::ABX, 4, true), // 0x1D
    Instruction(BaseInstruction::ASL, AddressingMode::ABX, 7, false), // 0x1E
    Instruction(BaseInstruction::SLO, AddressingMode::ABX, 7, false), // 0x1F
    //
    Instruction(BaseInstruction::JSR, AddressingMode::ABS, 6, false), // 0x20
    Instruction(BaseInstruction::AND, AddressingMode::IZX, 6, false), // 0x21
    Instruction(BaseInstruction::HLT, AddressingMode::IMP, 0, false), // 0x22
    Instruction(BaseInstruction::RLA, AddressingMode::IZX, 8, false), // 0x23
    Instruction(BaseInstruction::BIT, AddressingMode::ZP0, 3, false), // 0x24
    Instruction(BaseInstruction::AND, AddressingMode::ZP0, 3, false), // 0x25
    Instruction(BaseInstruction::ROL, AddressingMode::ZP0, 5, false), // 0x26
    Instruction(BaseInstruction::RLA, AddressingMode::ZP0, 5, false), // 0x27
    Instruction(BaseInstruction::PLP, AddressingMode::IMP, 4, false), // 0x28
    Instruction(BaseInstruction::AND, AddressingMode::IMM, 2, false), // 0x29
    Instruction(BaseInstruction::ROL, AddressingMode::IMP, 2, false), // 0x2A
    Instruction(BaseInstruction::ANC, AddressingMode::IMM, 2, false), // 0x2B
    Instruction(BaseInstruction::BIT, AddressingMode::ABS, 4, false), // 0x2C
    Instruction(BaseInstruction::AND, AddressingMode::ABS, 4, false), // 0x2D
    Instruction(BaseInstruction::ROL, AddressingMode::ABS, 6, false), // 0x2E
    Instruction(BaseInstruction::RLA, AddressingMode::ABS, 6, false), // 0x2F
    //
    Instruction(BaseInstruction::BMI, AddressingMode::REL, 2, true), // 0x30
    Instruction(BaseInstruction::AND, AddressingMode::IZY, 5, true), // 0x31
    Instruction(BaseInstruction::HLT, AddressingMode::IMP, 0, false), // 0x32
    Instruction(BaseInstruction::RLA, AddressingMode::IZY, 8, false), // 0x33
    Instruction(BaseInstruction::NOP, AddressingMode::ZPX, 4, false), // 0x34
    Instruction(BaseInstruction::AND, AddressingMode::ZPX, 4, false), // 0x35
    Instruction(BaseInstruction::ROL, AddressingMode::ZPX, 6, false), // 0x36
    Instruction(BaseInstruction::RLA, AddressingMode::ZPX, 6, false), // 0x37
    Instruction(BaseInstruction::SEC, AddressingMode::IMP, 2, false), // 0x38
    Instruction(BaseInstruction::AND, AddressingMode::ABY, 4, true), // 0x39
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 2, false), // 0x3A
    Instruction(BaseInstruction::RLA, AddressingMode::ABY, 7, false), // 0x3B
    Instruction(BaseInstruction::NOP, AddressingMode::ABX, 4, true), // 0x3C
    Instruction(BaseInstruction::AND, AddressingMode::ABX, 4, true), // 0x3D
    Instruction(BaseInstruction::ROL, AddressingMode::ABX, 7, false), // 0x3E
    Instruction(BaseInstruction::RLA, AddressingMode::ABX, 7, false), // 0x3F
    //
    Instruction(BaseInstruction::RTI, AddressingMode::IMP, 6, false), // 0x40
    Instruction(BaseInstruction::EOR, AddressingMode::IZX, 6, false), // 0x41
    Instruction(BaseInstruction::HLT, AddressingMode::IMP, 0, false), // 0x42
    Instruction(BaseInstruction::SRE, AddressingMode::IZX, 8, false), // 0x43
    Instruction(BaseInstruction::NOP, AddressingMode::ZP0, 3, false), // 0x44
    Instruction(BaseInstruction::EOR, AddressingMode::ZP0, 3, false), // 0x45
    Instruction(BaseInstruction::LSR, AddressingMode::ZP0, 5, false), // 0x46
    Instruction(BaseInstruction::SRE, AddressingMode::ZP0, 5, false), // 0x47
    Instruction(BaseInstruction::PHA, AddressingMode::IMP, 3, false), // 0x48
    Instruction(BaseInstruction::EOR, AddressingMode::IMM, 2, false), // 0x49
    Instruction(BaseInstruction::LSR, AddressingMode::IMP, 2, false), // 0x4A
    Instruction(BaseInstruction::ALR, AddressingMode::IMM, 2, false), // 0x4B
    Instruction(BaseInstruction::JMP, AddressingMode::ABS, 3, false), // 0x4C
    Instruction(BaseInstruction::EOR, AddressingMode::ABS, 4, false), // 0x4D
    Instruction(BaseInstruction::LSR, AddressingMode::ABS, 6, false), // 0x4E
    Instruction(BaseInstruction::SRE, AddressingMode::ABS, 6, false), // 0x4F
    //
    Instruction(BaseInstruction::BVC, AddressingMode::REL, 2, true), // 0x50
    Instruction(BaseInstruction::EOR, AddressingMode::IZY, 5, true), // 0x51
    Instruction(BaseInstruction::HLT, AddressingMode::IMP, 0, false), // 0x52
    Instruction(BaseInstruction::SRE, AddressingMode::IZY, 8, false), // 0x53
    Instruction(BaseInstruction::NOP, AddressingMode::ZPX, 4, false), // 0x54
    Instruction(BaseInstruction::EOR, AddressingMode::ZPX, 4, false), // 0x55
    Instruction(BaseInstruction::LSR, AddressingMode::ZPX, 6, false), // 0x56
    Instruction(BaseInstruction::SRE, AddressingMode::ZPX, 6, false), // 0x57
    Instruction(BaseInstruction::CLI, AddressingMode::IMP, 2, false), // 0x58
    Instruction(BaseInstruction::EOR, AddressingMode::ABY, 4, true), // 0x59
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 2, false), // 0x5A
    Instruction(BaseInstruction::SRE, AddressingMode::ABY, 7, false), // 0x5B
    Instruction(BaseInstruction::NOP, AddressingMode::ABX, 4, true), // 0x5C
    Instruction(BaseInstruction::EOR, AddressingMode::ABX, 4, true), // 0x5D
    Instruction(BaseInstruction::LSR, AddressingMode::ABX, 7, false), // 0x5E
    Instruction(BaseInstruction::SRE, AddressingMode::ABX, 7, false), // 0x5F
    //
    Instruction(BaseInstruction::RTS, AddressingMode::IMP, 6, false), // 0x60
    Instruction(BaseInstruction::ADC, AddressingMode::IZX, 6, false), // 0x61
    Instruction(BaseInstruction::HLT, AddressingMode::IMP, 0, false), // 0x62
    Instruction(BaseInstruction::RRA, AddressingMode::IZX, 8, false), // 0x63
    Instruction(BaseInstruction::NOP, AddressingMode::ZP0, 3, false), // 0x64
    Instruction(BaseInstruction::ADC, AddressingMode::ZP0, 3, false), // 0x65
    Instruction(BaseInstruction::ROR, AddressingMode::ZP0, 5, false), // 0x66
    Instruction(BaseInstruction::RRA, AddressingMode::ZP0, 5, false), // 0x67
    Instruction(BaseInstruction::PLA, AddressingMode::IMP, 4, false), // 0x68
    Instruction(BaseInstruction::ADC, AddressingMode::IMM, 2, false), // 0x69
    Instruction(BaseInstruction::ROR, AddressingMode::IMP, 2, false), // 0x6A
    Instruction(BaseInstruction::ARR, AddressingMode::IMM, 2, false), // 0x6B
    Instruction(BaseInstruction::JMP, AddressingMode::IND, 5, false), // 0x6C
    Instruction(BaseInstruction::ADC, AddressingMode::ABS, 4, false), // 0x6D
    Instruction(BaseInstruction::ROR, AddressingMode::ABS, 6, false), // 0x6E
    Instruction(BaseInstruction::RRA, AddressingMode::ABS, 6, false), // 0x6F
    //
    Instruction(BaseInstruction::BVS, AddressingMode::REL, 2, true), // 0x70
    Instruction(BaseInstruction::ADC, AddressingMode::IZY, 5, true), // 0x71
    Instruction(BaseInstruction::HLT, AddressingMode::IMP, 0, false), // 0x72
    Instruction(BaseInstruction::RRA, AddressingMode::IZY, 8, false), // 0x73
    Instruction(BaseInstruction::NOP, AddressingMode::ZPX, 4, false), // 0x74
    Instruction(BaseInstruction::ADC, AddressingMode::ZPX, 4, false), // 0x75
    Instruction(BaseInstruction::ROR, AddressingMode::ZPX, 6, false), // 0x76
    Instruction(BaseInstruction::RRA, AddressingMode::ZPX, 6, false), // 0x77
    Instruction(BaseInstruction::SEI, AddressingMode::IMP, 2, false), // 0x78
    Instruction(BaseInstruction::ADC, AddressingMode::ABY, 4, true), // 0x79
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 2, false), // 0x7A
    Instruction(BaseInstruction::RRA, AddressingMode::ABY, 7, false), // 0x7B
    Instruction(BaseInstruction::NOP, AddressingMode::ABX, 4, true), // 0x7C
    Instruction(BaseInstruction::ADC, AddressingMode::ABX, 4, true), // 0x7D
    Instruction(BaseInstruction::ROR, AddressingMode::ABX, 7, false), // 0x7E
    Instruction(BaseInstruction::RRA, AddressingMode::ABX, 7, false), // 0x7F
    //
    Instruction(BaseInstruction::NOP, AddressingMode::IMM, 2, false), // 0x80
    Instruction(BaseInstruction::STA, AddressingMode::IZX, 6, false), // 0x81
    Instruction(BaseInstruction::NOP, AddressingMode::IMM, 2, false), // 0x82
    Instruction(BaseInstruction::SAX, AddressingMode::IZX, 6, false), // 0x83
    Instruction(BaseInstruction::STY, AddressingMode::ZP0, 3, false), // 0x84
    Instruction(BaseInstruction::STA, AddressingMode::ZP0, 3, false), // 0x85
    Instruction(BaseInstruction::STX, AddressingMode::ZP0, 3, false), // 0x86
    Instruction(BaseInstruction::SAX, AddressingMode::ZP0, 3, false), // 0x87
    Instruction(BaseInstruction::DEY, AddressingMode::IMP, 2, false), // 0x88
    Instruction(BaseInstruction::NOP, AddressingMode::IMM, 2, false), // 0x89
    Instruction(BaseInstruction::TXA, AddressingMode::IMP, 2, false), // 0x8A
    Instruction(BaseInstruction::XAA, AddressingMode::IMM, 2, false), // 0x8B
    Instruction(BaseInstruction::STY, AddressingMode::ABS, 4, false), // 0x8C
    Instruction(BaseInstruction::STA, AddressingMode::ABS, 4, false), // 0x8D
    Instruction(BaseInstruction::STX, AddressingMode::ABS, 4, false), // 0x8E
    Instruction(BaseInstruction::SAX, AddressingMode::ABS, 4, false), // 0x8F
    //
    Instruction(BaseInstruction::BCC, AddressingMode::REL, 2, true), // 0x90
    Instruction(BaseInstruction::STA, AddressingMode::IZY, 6, false), // 0x91
    Instruction(BaseInstruction::HLT, AddressingMode::IMP, 0, false), // 0x92
    Instruction(BaseInstruction::AHX, AddressingMode::IZY, 6, false), // 0x93
    Instruction(BaseInstruction::STY, AddressingMode::ZPX, 4, false), // 0x94
    Instruction(BaseInstruction::STA, AddressingMode::ZPX, 4, false), // 0x95
    Instruction(BaseInstruction::STX, AddressingMode::ZPY, 4, false), // 0x96
    Instruction(BaseInstruction::SAX, AddressingMode::ZPY, 4, false), // 0x97
    Instruction(BaseInstruction::TYA, AddressingMode::IMP, 2, false), // 0x98
    Instruction(BaseInstruction::STA, AddressingMode::ABY, 5, false), // 0x99
    Instruction(BaseInstruction::TXS, AddressingMode::IMP, 2, false), // 0x9A
    Instruction(BaseInstruction::TAS, AddressingMode::ABY, 5, false), // 0x9B
    Instruction(BaseInstruction::SHY, AddressingMode::ABX, 5, false), // 0x9C
    Instruction(BaseInstruction::STA, AddressingMode::ABX, 5, false), // 0x9D
    Instruction(BaseInstruction::SHX, AddressingMode::ABY, 5, false), // 0x9E
    Instruction(BaseInstruction::AHX, AddressingMode::ABY, 5, false), // 0x9F
    //
    Instruction(BaseInstruction::LDY, AddressingMode::IMM, 2, false), // 0xA0
    Instruction(BaseInstruction::LDA, AddressingMode::IZX, 6, false), // 0xA1
    Instruction(BaseInstruction::LDX, AddressingMode::IMM, 2, false), // 0xA2
    Instruction(BaseInstruction::LAX, AddressingMode::IZX, 6, false), // 0xA3
    Instruction(BaseInstruction::LDY, AddressingMode::ZP0, 3, false), // 0xA4
    Instruction(BaseInstruction::LDA, AddressingMode::ZP0, 3, false), // 0xA5
    Instruction(BaseInstruction::LDX, AddressingMode::ZP0, 3, false), // 0xA6
    Instruction(BaseInstruction::LAX, AddressingMode::ZP0, 3, false), // 0xA7
    Instruction(BaseInstruction::TAY, AddressingMode::IMP, 2, false), // 0xA8
    Instruction(BaseInstruction::LDA, AddressingMode::IMM, 2, false), // 0xA9
    Instruction(BaseInstruction::TAX, AddressingMode::IMP, 2, false), // 0xAA
    Instruction(BaseInstruction::LAX, AddressingMode::IMM, 2, false), // 0xAB
    Instruction(BaseInstruction::LDY, AddressingMode::ABS, 4, false), // 0xAC
    Instruction(BaseInstruction::LDA, AddressingMode::ABS, 4, false), // 0xAD
    Instruction(BaseInstruction::LDX, AddressingMode::ABS, 4, false), // 0xAE
    Instruction(BaseInstruction::LAX, AddressingMode::ABS, 4, false), // 0xAF
    //
    Instruction(BaseInstruction::BCS, AddressingMode::REL, 2, true), // 0xB0
    Instruction(BaseInstruction::LDA, AddressingMode::IZY, 5, true), // 0xB1
    Instruction(BaseInstruction::HLT, AddressingMode::IMP, 0, false), // 0xB2
    Instruction(BaseInstruction::LAX, AddressingMode::IZY, 5, true), // 0xB3
    Instruction(BaseInstruction::LDY, AddressingMode::ZPX, 4, false), // 0xB4
    Instruction(BaseInstruction::LDA, AddressingMode::ZPX, 4, false), // 0xB5
    Instruction(BaseInstruction::LDX, AddressingMode::ZPY, 4, false), // 0xB6
    Instruction(BaseInstruction::LAX, AddressingMode::ZPY, 4, false), // 0xB7
    Instruction(BaseInstruction::CLV, AddressingMode::IMP, 2, false), // 0xB8
    Instruction(BaseInstruction::LDA, AddressingMode::ABY, 4, true), // 0xB9
    Instruction(BaseInstruction::TSX, AddressingMode::IMP, 2, false), // 0xBA
    Instruction(BaseInstruction::LAS, AddressingMode::ABY, 4, true), // 0xBB
    Instruction(BaseInstruction::LDY, AddressingMode::ABX, 4, true), // 0xBC
    Instruction(BaseInstruction::LDA, AddressingMode::ABX, 4, true), // 0xBD
    Instruction(BaseInstruction::LDX, AddressingMode::ABY, 4, true), // 0xBE
    Instruction(BaseInstruction::LAX, AddressingMode::ABY, 4, true), // 0xBF
    //
    Instruction(BaseInstruction::CPY, AddressingMode::IMM, 2, false), // 0xC0
    Instruction(BaseInstruction::CMP, AddressingMode::IZX, 6, false), // 0xC1
    Instruction(BaseInstruction::NOP, AddressingMode::IMM, 2, false), // 0xC2
    Instruction(BaseInstruction::DCP, AddressingMode::IZX, 8, false), // 0xC3
    Instruction(BaseInstruction::CPY, AddressingMode::ZP0, 3, false), // 0xC4
    Instruction(BaseInstruction::CMP, AddressingMode::ZP0, 3, false), // 0xC5
    Instruction(BaseInstruction::DEC, AddressingMode::ZP0, 5, false), // 0xC6
    Instruction(BaseInstruction::DCP, AddressingMode::ZP0, 5, false), // 0xC7
    Instruction(BaseInstruction::INY, AddressingMode::IMP, 2, false), // 0xC8
    Instruction(BaseInstruction::CMP, AddressingMode::IMM, 2, false), // 0xC9
    Instruction(BaseInstruction::DEX, AddressingMode::IMP, 2, false), // 0xCA
    Instruction(BaseInstruction::AXS, AddressingMode::IMM, 2, false), // 0xCB
    Instruction(BaseInstruction::CPY, AddressingMode::ABS, 4, false), // 0xCC
    Instruction(BaseInstruction::CMP, AddressingMode::ABS, 4, false), // 0xCD
    Instruction(BaseInstruction::DEC, AddressingMode::ABS, 6, false), // 0xCE
    Instruction(BaseInstruction::DCP, AddressingMode::ABS, 6, false), // 0xCF
    //
    Instruction(BaseInstruction::BNE, AddressingMode::REL, 2, true), // 0xD0
    Instruction(BaseInstruction::CMP, AddressingMode::IZY, 5, true), // 0xD1
    Instruction(BaseInstruction::HLT, AddressingMode::IMP, 0, false), // 0xD2
    Instruction(BaseInstruction::DCP, AddressingMode::IZY, 8, false), // 0xD3
    Instruction(BaseInstruction::NOP, AddressingMode::ZPX, 4, false), // 0xD4
    Instruction(BaseInstruction::CMP, AddressingMode::ZPX, 4, false), // 0xD5
    Instruction(BaseInstruction::DEC, AddressingMode::ZPX, 6, false), // 0xD6
    Instruction(BaseInstruction::DCP, AddressingMode::ZPX, 6, false), // 0xD7
    Instruction(BaseInstruction::CLD, AddressingMode::IMP, 2, false), // 0xD8
    Instruction(BaseInstruction::CMP, AddressingMode::ABY, 4, true), // 0xD9
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 2, false), // 0xDA
    Instruction(BaseInstruction::DCP, AddressingMode::ABY, 7, false), // 0xDB
    Instruction(BaseInstruction::NOP, AddressingMode::ABX, 4, true), // 0xDC
    Instruction(BaseInstruction::CMP, AddressingMode::ABX, 4, true), // 0xDD
    Instruction(BaseInstruction::DEC, AddressingMode::ABX, 7, false), // 0xDE
    Instruction(BaseInstruction::DCP, AddressingMode::ABX, 7, false), // 0xDF
    //
    Instruction(BaseInstruction::CPX, AddressingMode::IMM, 2, false), // 0xE0
    Instruction(BaseInstruction::SBC, AddressingMode::IZX, 6, false), // 0xE1
    Instruction(BaseInstruction::NOP, AddressingMode::IMM, 2, false), // 0xE2
    Instruction(BaseInstruction::ISC, AddressingMode::IZX, 8, false), // 0xE3
    Instruction(BaseInstruction::CPX, AddressingMode::ZP0, 3, false), // 0xE4
    Instruction(BaseInstruction::SBC, AddressingMode::ZP0, 3, false), // 0xE5
    Instruction(BaseInstruction::INC, AddressingMode::ZP0, 5, false), // 0xE6
    Instruction(BaseInstruction::ISC, AddressingMode::ZP0, 5, false), // 0xE7
    Instruction(BaseInstruction::INX, AddressingMode::IMP, 2, false), // 0xE8
    Instruction(BaseInstruction::SBC, AddressingMode::IMM, 2, false), // 0xE9
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 2, false), // 0xEA
    Instruction(BaseInstruction::SBC, AddressingMode::IMM, 2, false), // 0xEB
    Instruction(BaseInstruction::CPX, AddressingMode::ABS, 4, false), // 0xEC
    Instruction(BaseInstruction::SBC, AddressingMode::ABS, 4, false), // 0xED
    Instruction(BaseInstruction::INC, AddressingMode::ABS, 6, false), // 0xEE
    Instruction(BaseInstruction::ISC, AddressingMode::ABS, 6, false), // 0xEF
    //
    Instruction(BaseInstruction::BEQ, AddressingMode::REL, 2, true), // 0xF0
    Instruction(BaseInstruction::SBC, AddressingMode::IZY, 5, true), // 0xF1
    Instruction(BaseInstruction::HLT, AddressingMode::IMP, 0, false), // 0xF2
    Instruction(BaseInstruction::ISC, AddressingMode::IZY, 8, false), // 0xF3
    Instruction(BaseInstruction::NOP, AddressingMode::ZPX, 4, false), // 0xF4
    Instruction(BaseInstruction::SBC, AddressingMode::ZPX, 4, false), // 0xF5
    Instruction(BaseInstruction::INC, AddressingMode::ZPX, 6, false), // 0xF6
    Instruction(BaseInstruction::ISC, AddressingMode::ZPX, 6, false), // 0xF7
    Instruction(BaseInstruction::SED, AddressingMode::IMP, 2, false), // 0xF8
    Instruction(BaseInstruction::SBC, AddressingMode::ABY, 4, true), // 0xF9
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 2, false), // 0xFA
    Instruction(BaseInstruction::ISC, AddressingMode::ABY, 7, false), // 0xFB
    Instruction(BaseInstruction::NOP, AddressingMode::ABX, 4, true), // 0xFC
    Instruction(BaseInstruction::SBC, AddressingMode::ABX, 4, true), // 0xFD
    Instruction(BaseInstruction::INC, AddressingMode::ABX, 7, false), // 0xFE
    Instruction(BaseInstruction::ISC, AddressingMode::ABX, 7, false), // 0xFF
];

const INSTRUCTION_LOOKUP_65C02: [Instruction; 256] = [
    Instruction(BaseInstruction::BRK, AddressingMode::IMP, 7, false), // 0x00
    Instruction(BaseInstruction::ORA, AddressingMode::IZX, 6, false), // 0x01
    Instruction(BaseInstruction::NOP, AddressingMode::IMM, 2, false), // 0x02
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 1, false), // 0x03
    Instruction(BaseInstruction::TSB, AddressingMode::ZP0, 5, false), // 0x04
    Instruction(BaseInstruction::ORA, AddressingMode::ZP0, 3, false), // 0x05
    Instruction(BaseInstruction::ASL, AddressingMode::ZP0, 5, false), // 0x06
    Instruction(BaseInstruction::RMB0, AddressingMode::ZP0, 5, false), // 0x07
    Instruction(BaseInstruction::PHP, AddressingMode::IMP, 3, false), // 0x08
    Instruction(BaseInstruction::ORA, AddressingMode::IMM, 2, false), // 0x09
    Instruction(BaseInstruction::ASL, AddressingMode::IMP, 2, false), // 0x0A
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 1, false), // 0x0B
    Instruction(BaseInstruction::TSB, AddressingMode::ABS, 6, false), // 0x0C
    Instruction(BaseInstruction::ORA, AddressingMode::ABS, 4, false), // 0x0D
    Instruction(BaseInstruction::ASL, AddressingMode::ABS, 6, false), // 0x0E
    Instruction(BaseInstruction::BBR0, AddressingMode::ZPR, 5, false), // 0x0F
    //
    Instruction(BaseInstruction::BPL, AddressingMode::REL, 2, true), // 0x10
    Instruction(BaseInstruction::ORA, AddressingMode::IZY, 5, true), // 0x11
    Instruction(BaseInstruction::ORA, AddressingMode::IZP, 5, false), // 0x12
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 1, false), // 0x13
    Instruction(BaseInstruction::TRB, AddressingMode::ZP0, 5, false), // 0x14
    Instruction(BaseInstruction::ORA, AddressingMode::ZPX, 4, false), // 0x15
    Instruction(BaseInstruction::ASL, AddressingMode::ZPX, 6, false), // 0x16
    Instruction(BaseInstruction::RMB1, AddressingMode::ZP0, 5, false), // 0x17
    Instruction(BaseInstruction::CLC, AddressingMode::IMP, 2, false), // 0x18
    Instruction(BaseInstruction::ORA, AddressingMode::ABY, 4, true), // 0x19
    Instruction(BaseInstruction::INC, AddressingMode::IMP, 2, false), // 0x1A
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 1, false), // 0x1B
    Instruction(BaseInstruction::TRB, AddressingMode::ABS, 6, false), // 0x1C
    Instruction(BaseInstruction::ORA, AddressingMode::ABX, 4, true), // 0x1D
    Instruction(BaseInstruction::ASL, AddressingMode::ABX, 6, true), // 0x1E
    Instruction(BaseInstruction::BBR1, AddressingMode::ZPR, 5, false), // 0x1F
    //
    Instruction(BaseInstruction::JSR, AddressingMode::ABS, 6, false), // 0x20
    Instruction(BaseInstruction::AND, AddressingMode::IZX, 6, false), // 0x21
    Instruction(BaseInstruction::NOP, AddressingMode::IMM, 2, false), // 0x22
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 1, false), // 0x23
    Instruction(BaseInstruction::BIT, AddressingMode::ZP0, 3, false), // 0x24
    Instruction(BaseInstruction::AND, AddressingMode::ZP0, 3, false), // 0x25
    Instruction(BaseInstruction::ROL, AddressingMode::ZP0, 5, false), // 0x26
    Instruction(BaseInstruction::RMB2, AddressingMode::ZP0, 5, false), // 0x27
    Instruction(BaseInstruction::PLP, AddressingMode::IMP, 4, false), // 0x28
    Instruction(BaseInstruction::AND, AddressingMode::IMM, 2, false), // 0x29
    Instruction(BaseInstruction::ROL, AddressingMode::IMP, 2, false), // 0x2A
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 1, false), // 0x2B
    Instruction(BaseInstruction::BIT, AddressingMode::ABS, 4, false), // 0x2C
    Instruction(BaseInstruction::AND, AddressingMode::ABS, 4, false), // 0x2D
    Instruction(BaseInstruction::ROL, AddressingMode::ABS, 6, false), // 0x2E
    Instruction(BaseInstruction::BBR2, AddressingMode::ZPR, 5, false), // 0x2F
    //
    Instruction(BaseInstruction::BMI, AddressingMode::REL, 2, true), // 0x30
    Instruction(BaseInstruction::AND, AddressingMode::IZY, 5, true), // 0x31
    Instruction(BaseInstruction::AND, AddressingMode::IZP, 5, false), // 0x32
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 1, false), // 0x33
    Instruction(BaseInstruction::BIT, AddressingMode::ZPX, 4, false), // 0x34
    Instruction(BaseInstruction::AND, AddressingMode::ZPX, 4, false), // 0x35
    Instruction(BaseInstruction::ROL, AddressingMode::ZPX, 6, false), // 0x36
    Instruction(BaseInstruction::RMB3, AddressingMode::ZP0, 5, false), // 0x37
    Instruction(BaseInstruction::SEC, AddressingMode::IMP, 2, false), // 0x38
    Instruction(BaseInstruction::AND, AddressingMode::ABY, 4, true), // 0x39
    Instruction(BaseInstruction::DEC, AddressingMode::IMP, 2, false), // 0x3A
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 1, false), // 0x3B
    Instruction(BaseInstruction::BIT, AddressingMode::ABX, 4, true), // 0x3C
    Instruction(BaseInstruction::AND, AddressingMode::ABX, 4, true), // 0x3D
    Instruction(BaseInstruction::ROL, AddressingMode::ABX, 6, true), // 0x3E
    Instruction(BaseInstruction::BBR3, AddressingMode::ZPR, 5, false), // 0x3F
    //
    Instruction(BaseInstruction::RTI, AddressingMode::IMP, 6, false), // 0x40
    Instruction(BaseInstruction::EOR, AddressingMode::IZX, 6, false), // 0x41
    Instruction(BaseInstruction::NOP, AddressingMode::IMM, 2, false), // 0x42
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 1, false), // 0x43
    Instruction(BaseInstruction::NOP, AddressingMode::ZP0, 3, false), // 0x44
    Instruction(BaseInstruction::EOR, AddressingMode::ZP0, 3, false), // 0x45
    Instruction(BaseInstruction::LSR, AddressingMode::ZP0, 5, false), // 0x46
    Instruction(BaseInstruction::RMB4, AddressingMode::ZP0, 5, false), // 0x47
    Instruction(BaseInstruction::PHA, AddressingMode::IMP, 3, false), // 0x48
    Instruction(BaseInstruction::EOR, AddressingMode::IMM, 2, false), // 0x49
    Instruction(BaseInstruction::LSR, AddressingMode::IMP, 2, false), // 0x4A
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 1, false), // 0x4B
    Instruction(BaseInstruction::JMP, AddressingMode::ABS, 3, false), // 0x4C
    Instruction(BaseInstruction::EOR, AddressingMode::ABS, 4, false), // 0x4D
    Instruction(BaseInstruction::LSR, AddressingMode::ABS, 6, false), // 0x4E
    Instruction(BaseInstruction::BBR4, AddressingMode::ZPR, 5, false), // 0x4F
    //
    Instruction(BaseInstruction::BVC, AddressingMode::REL, 2, true), // 0x50
    Instruction(BaseInstruction::EOR, AddressingMode::IZY, 5, true), // 0x51
    Instruction(BaseInstruction::EOR, AddressingMode::IZP, 5, false), // 0x52
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 1, false), // 0x53
    Instruction(BaseInstruction::NOP, AddressingMode::ZPX, 4, false), // 0x54
    Instruction(BaseInstruction::EOR, AddressingMode::ZPX, 4, false), // 0x55
    Instruction(BaseInstruction::LSR, AddressingMode::ZPX, 6, false), // 0x56
    Instruction(BaseInstruction::RMB5, AddressingMode::ZP0, 5, false), // 0x57
    Instruction(BaseInstruction::CLI, AddressingMode::IMP, 2, false), // 0x58
    Instruction(BaseInstruction::EOR, AddressingMode::ABY, 4, true), // 0x59
    Instruction(BaseInstruction::PHY, AddressingMode::IMP, 3, false), // 0x5A
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 1, false), // 0x5B
    Instruction(BaseInstruction::NOP, AddressingMode::ABS, 8, false), // 0x5C
    Instruction(BaseInstruction::EOR, AddressingMode::ABX, 4, true), // 0x5D
    Instruction(BaseInstruction::LSR, AddressingMode::ABX, 6, true), // 0x5E
    Instruction(BaseInstruction::BBR5, AddressingMode::ZPR, 5, false), // 0x5F
    //
    Instruction(BaseInstruction::RTS, AddressingMode::IMP, 6, false), // 0x60
    Instruction(BaseInstruction::ADC, AddressingMode::IZX, 6, false), // 0x61
    Instruction(BaseInstruction::NOP, AddressingMode::IMM, 2, false), // 0x62
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 1, false), // 0x63
    Instruction(BaseInstruction::STZ, AddressingMode::ZP0, 3, false), // 0x64
    Instruction(BaseInstruction::ADC, AddressingMode::ZP0, 3, false), // 0x65
    Instruction(BaseInstruction::ROR, AddressingMode::ZP0, 5, false), // 0x66
    Instruction(BaseInstruction::RMB6, AddressingMode::ZP0, 5, false), // 0x67
    Instruction(BaseInstruction::PLA, AddressingMode::IMP, 4, false), // 0x68
    Instruction(BaseInstruction::ADC, AddressingMode::IMM, 2, false), // 0x69
    Instruction(BaseInstruction::ROR, AddressingMode::IMP, 2, false), // 0x6A
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 1, false), // 0x6B
    Instruction(BaseInstruction::JMP, AddressingMode::IND, 6, false), // 0x6C
    Instruction(BaseInstruction::ADC, AddressingMode::ABS, 4, false), // 0x6D
    Instruction(BaseInstruction::ROR, AddressingMode::ABS, 6, false), // 0x6E
    Instruction(BaseInstruction::BBR6, AddressingMode::ZPR, 5, false), // 0x6F
    //
    Instruction(BaseInstruction::BVS, AddressingMode::REL, 2, true), // 0x70
    Instruction(BaseInstruction::ADC, AddressingMode::IZY, 5, true), // 0x71
    Instruction(BaseInstruction::ADC, AddressingMode::IZP, 5, false), // 0x72
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 1, false), // 0x73
    Instruction(BaseInstruction::STZ, AddressingMode::ZPX, 4, false), // 0x74
    Instruction(BaseInstruction::ADC, AddressingMode::ZPX, 4, false), // 0x75
    Instruction(BaseInstruction::ROR, AddressingMode::ZPX, 6, false), // 0x76
    Instruction(BaseInstruction::RMB7, AddressingMode::ZP0, 5, false), // 0x77
    Instruction(BaseInstruction::SEI, AddressingMode::IMP, 2, false), // 0x78
    Instruction(BaseInstruction::ADC, AddressingMode::ABY, 4, true), // 0x79
    Instruction(BaseInstruction::PLY, AddressingMode::IMP, 4, false), // 0x7A
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 1, false), // 0x7B
    Instruction(BaseInstruction::JMP, AddressingMode::IAX, 6, false), // 0x7C
    Instruction(BaseInstruction::ADC, AddressingMode::ABX, 4, true), // 0x7D
    Instruction(BaseInstruction::ROR, AddressingMode::ABX, 6, true), // 0x7E
    Instruction(BaseInstruction::BBR7, AddressingMode::ZPR, 5, false), // 0x7F
    //
    Instruction(BaseInstruction::BRA, AddressingMode::REL, 3, true), // 0x80
    Instruction(BaseInstruction::STA, AddressingMode::IZX, 6, false), // 0x81
    Instruction(BaseInstruction::NOP, AddressingMode::IMM, 2, false), // 0x82
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 1, false), // 0x83
    Instruction(BaseInstruction::STY, AddressingMode::ZP0, 3, false), // 0x84
    Instruction(BaseInstruction::STA, AddressingMode::ZP0, 3, false), // 0x85
    Instruction(BaseInstruction::STX, AddressingMode::ZP0, 3, false), // 0x86
    Instruction(BaseInstruction::SMB0, AddressingMode::ZP0, 5, false), // 0x87
    Instruction(BaseInstruction::DEY, AddressingMode::IMP, 2, false), // 0x88
    Instruction(BaseInstruction::BIT, AddressingMode::IMM, 2, false), // 0x89
    Instruction(BaseInstruction::TXA, AddressingMode::IMP, 2, false), // 0x8A
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 1, false), // 0x8B
    Instruction(BaseInstruction::STY, AddressingMode::ABS, 4, false), // 0x8C
    Instruction(BaseInstruction::STA, AddressingMode::ABS, 4, false), // 0x8D
    Instruction(BaseInstruction::STX, AddressingMode::ABS, 4, false), // 0x8E
    Instruction(BaseInstruction::BBS0, AddressingMode::ZPR, 5, false), // 0x8F
    //
    Instruction(BaseInstruction::BCC, AddressingMode::REL, 2, true), // 0x90
    Instruction(BaseInstruction::STA, AddressingMode::IZY, 6, false), // 0x91
    Instruction(BaseInstruction::STA, AddressingMode::IZP, 5, false), // 0x92
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 1, false), // 0x93
    Instruction(BaseInstruction::STY, AddressingMode::ZPX, 4, false), // 0x94
    Instruction(BaseInstruction::STA, AddressingMode::ZPX, 4, false), // 0x95
    Instruction(BaseInstruction::STX, AddressingMode::ZP0, 4, false), // 0x96
    Instruction(BaseInstruction::SMB1, AddressingMode::ZP0, 5, false), // 0x97
    Instruction(BaseInstruction::TYA, AddressingMode::IMP, 2, false), // 0x98
    Instruction(BaseInstruction::STA, AddressingMode::ABY, 5, false), // 0x99
    Instruction(BaseInstruction::TXS, AddressingMode::IMP, 2, false), // 0x9A
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 1, false), // 0x9B
    Instruction(BaseInstruction::STZ, AddressingMode::ABS, 4, false), // 0x9C
    Instruction(BaseInstruction::STA, AddressingMode::ABX, 5, false), // 0x9D
    Instruction(BaseInstruction::STZ, AddressingMode::ABX, 5, false), // 0x9E
    Instruction(BaseInstruction::BBS1, AddressingMode::ZPR, 5, false), // 0x9F
    //
    Instruction(BaseInstruction::LDY, AddressingMode::IMM, 2, false), // 0xA0
    Instruction(BaseInstruction::LDA, AddressingMode::IZX, 6, false), // 0xA1
    Instruction(BaseInstruction::LDX, AddressingMode::IMM, 2, false), // 0xA2
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 1, false), // 0xA3
    Instruction(BaseInstruction::LDY, AddressingMode::ZP0, 3, false), // 0xA4
    Instruction(BaseInstruction::LDA, AddressingMode::ZP0, 3, false), // 0xA5
    Instruction(BaseInstruction::LDX, AddressingMode::ZP0, 3, false), // 0xA6
    Instruction(BaseInstruction::SMB2, AddressingMode::ZP0, 5, false), // 0xA7
    Instruction(BaseInstruction::TAY, AddressingMode::IMP, 2, false), // 0xA8
    Instruction(BaseInstruction::LDA, AddressingMode::IMM, 2, false), // 0xA9
    Instruction(BaseInstruction::TAX, AddressingMode::IMP, 2, false), // 0xAA
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 1, false), // 0xAB
    Instruction(BaseInstruction::LDY, AddressingMode::ABS, 4, false), // 0xAC
    Instruction(BaseInstruction::LDA, AddressingMode::ABS, 4, false), // 0xAD
    Instruction(BaseInstruction::LDX, AddressingMode::ABS, 4, false), // 0xAE
    Instruction(BaseInstruction::BBS2, AddressingMode::ZPR, 5, false), // 0xAF
    //
    Instruction(BaseInstruction::BCS, AddressingMode::REL, 2, true), // 0xB0
    Instruction(BaseInstruction::LDA, AddressingMode::IZY, 5, true), // 0xB1
    Instruction(BaseInstruction::LDA, AddressingMode::IZP, 5, false), // 0xB2
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 1, false), // 0xB3
    Instruction(BaseInstruction::LDY, AddressingMode::ZPX, 4, false), // 0xB4
    Instruction(BaseInstruction::LDA, AddressingMode::ZPX, 4, false), // 0xB5
    Instruction(BaseInstruction::LDX, AddressingMode::ZP0, 4, false), // 0xB6
    Instruction(BaseInstruction::SMB3, AddressingMode::ZP0, 5, false), // 0xB7
    Instruction(BaseInstruction::CLV, AddressingMode::IMP, 2, false), // 0xB8
    Instruction(BaseInstruction::LDA, AddressingMode::ABY, 4, true), // 0xB9
    Instruction(BaseInstruction::TSX, AddressingMode::IMP, 2, false), // 0xBA
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 1, false), // 0xBB
    Instruction(BaseInstruction::LDY, AddressingMode::ABX, 4, true), // 0xBC
    Instruction(BaseInstruction::LDA, AddressingMode::ABX, 4, true), // 0xBD
    Instruction(BaseInstruction::LDX, AddressingMode::ABY, 4, true), // 0xBE
    Instruction(BaseInstruction::BBS3, AddressingMode::ZPR, 5, false), // 0xBF
    //
    Instruction(BaseInstruction::CPY, AddressingMode::IMM, 2, false), // 0xC0
    Instruction(BaseInstruction::CMP, AddressingMode::IZX, 6, false), // 0xC1
    Instruction(BaseInstruction::NOP, AddressingMode::IMM, 2, false), // 0xC2
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 1, false), // 0xC3
    Instruction(BaseInstruction::CPY, AddressingMode::ZP0, 3, false), // 0xC4
    Instruction(BaseInstruction::CMP, AddressingMode::ZP0, 3, false), // 0xC5
    Instruction(BaseInstruction::DEC, AddressingMode::ZP0, 5, false), // 0xC6
    Instruction(BaseInstruction::SMB4, AddressingMode::ZP0, 5, false), // 0xC7
    Instruction(BaseInstruction::INY, AddressingMode::IMP, 2, false), // 0xC8
    Instruction(BaseInstruction::CMP, AddressingMode::IMM, 2, false), // 0xC9
    Instruction(BaseInstruction::DEX, AddressingMode::IMP, 2, false), // 0xCA
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 3, false), // 0xCB
    Instruction(BaseInstruction::CPY, AddressingMode::ABS, 4, false), // 0xCC
    Instruction(BaseInstruction::CMP, AddressingMode::ABS, 4, false), // 0xCD
    Instruction(BaseInstruction::DEC, AddressingMode::ABS, 6, false), // 0xCE
    Instruction(BaseInstruction::BBS4, AddressingMode::ZPR, 5, false), // 0xCF
    //
    Instruction(BaseInstruction::BNE, AddressingMode::REL, 2, true), // 0xD0
    Instruction(BaseInstruction::CMP, AddressingMode::IZY, 5, true), // 0xD1
    Instruction(BaseInstruction::CMP, AddressingMode::IZP, 5, false), // 0xD2
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 1, false), // 0xD3
    Instruction(BaseInstruction::NOP, AddressingMode::ZPX, 4, false), // 0xD4
    Instruction(BaseInstruction::CMP, AddressingMode::ZPX, 4, false), // 0xD5
    Instruction(BaseInstruction::DEC, AddressingMode::ZPX, 6, false), // 0xD6
    Instruction(BaseInstruction::SMB5, AddressingMode::ZP0, 5, false), // 0xD7
    Instruction(BaseInstruction::CLD, AddressingMode::IMP, 2, false), // 0xD8
    Instruction(BaseInstruction::CMP, AddressingMode::ABY, 4, true), // 0xD9
    Instruction(BaseInstruction::PHX, AddressingMode::IMP, 3, false), // 0xDA
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 3, false), // 0xDB
    Instruction(BaseInstruction::NOP, AddressingMode::ABS, 4, false), // 0xDC
    Instruction(BaseInstruction::CMP, AddressingMode::ABX, 4, true), // 0xDD
    Instruction(BaseInstruction::DEC, AddressingMode::ABX, 7, false), // 0xDE
    Instruction(BaseInstruction::BBS5, AddressingMode::ZPR, 5, false), // 0xDF
    //
    Instruction(BaseInstruction::CPX, AddressingMode::IMM, 2, false), // 0xE0
    Instruction(BaseInstruction::SBC, AddressingMode::IZX, 6, false), // 0xE1
    Instruction(BaseInstruction::NOP, AddressingMode::IMM, 2, false), // 0xE2
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 1, false), // 0xE3
    Instruction(BaseInstruction::CPX, AddressingMode::ZP0, 3, false), // 0xE4
    Instruction(BaseInstruction::SBC, AddressingMode::ZP0, 3, false), // 0xE5
    Instruction(BaseInstruction::INC, AddressingMode::ZP0, 5, false), // 0xE6
    Instruction(BaseInstruction::SMB6, AddressingMode::ZP0, 5, false), // 0xE7
    Instruction(BaseInstruction::INX, AddressingMode::IMP, 2, false), // 0xE8
    Instruction(BaseInstruction::SBC, AddressingMode::IMM, 2, false), // 0xE9
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 2, false), // 0xEA
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 1, false), // 0xEB
    Instruction(BaseInstruction::CPX, AddressingMode::ABS, 4, false), // 0xEC
    Instruction(BaseInstruction::SBC, AddressingMode::ABS, 4, false), // 0xED
    Instruction(BaseInstruction::INC, AddressingMode::ABS, 6, false), // 0xEE
    Instruction(BaseInstruction::BBS6, AddressingMode::ZPR, 5, false), // 0xEF
    //
    Instruction(BaseInstruction::BEQ, AddressingMode::REL, 2, true), // 0xF0
    Instruction(BaseInstruction::SBC, AddressingMode::IZY, 5, true), // 0xF1
    Instruction(BaseInstruction::SBC, AddressingMode::IZP, 5, false), // 0xF2
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 1, false), // 0xF3
    Instruction(BaseInstruction::NOP, AddressingMode::ZPX, 4, false), // 0xF4
    Instruction(BaseInstruction::SBC, AddressingMode::ZPX, 4, false), // 0xF5
    Instruction(BaseInstruction::INC, AddressingMode::ZPX, 6, false), // 0xF6
    Instruction(BaseInstruction::SMB7, AddressingMode::ZP0, 5, false), // 0xF7
    Instruction(BaseInstruction::SED, AddressingMode::IMP, 2, false), // 0xF8
    Instruction(BaseInstruction::SBC, AddressingMode::ABY, 4, true), // 0xF9
    Instruction(BaseInstruction::PLX, AddressingMode::IMP, 4, false), // 0xFA
    Instruction(BaseInstruction::NOP, AddressingMode::IMP, 1, false), // 0xFB
    Instruction(BaseInstruction::NOP, AddressingMode::ABS, 4, false), // 0xFC
    Instruction(BaseInstruction::SBC, AddressingMode::ABX, 4, true), // 0xFD
    Instruction(BaseInstruction::INC, AddressingMode::ABX, 7, false), // 0xFE
    Instruction(BaseInstruction::BBS7, AddressingMode::ZPR, 5, false), // 0xFF
];

impl<'a> Cpu6502<'a> {
    #[inline]
    fn execute_lda(&mut self, data: ExecutionData) -> u32 {
        self.a = data.read_data(self);
        self.set_zn_flags(self.a);
        0
    }

    #[inline]
    fn execute_ldx(&mut self, data: ExecutionData) -> u32 {
        self.x = data.read_data(self);
        self.set_zn_flags(self.x);
        0
    }

    #[inline]
    fn execute_ldy(&mut self, data: ExecutionData) -> u32 {
        self.y = data.read_data(self);
        self.set_zn_flags(self.y);
        0
    }

    #[inline]
    fn execute_sta(&mut self, data: ExecutionData) -> u32 {
        data.write_data(self, self.a);
        0
    }

    #[inline]
    fn execute_stx(&mut self, data: ExecutionData) -> u32 {
        data.write_data(self, self.x);
        0
    }

    #[inline]
    fn execute_sty(&mut self, data: ExecutionData) -> u32 {
        data.write_data(self, self.y);
        0
    }

    #[inline]
    fn execute_tax(&mut self) -> u32 {
        self.x = self.a;
        self.set_zn_flags(self.x);
        0
    }

    #[inline]
    fn execute_tay(&mut self) -> u32 {
        self.y = self.a;
        self.set_zn_flags(self.y);
        0
    }

    #[inline]
    fn execute_txa(&mut self) -> u32 {
        self.a = self.x;
        self.set_zn_flags(self.a);
        0
    }

    #[inline]
    fn execute_tya(&mut self) -> u32 {
        self.a = self.y;
        self.set_zn_flags(self.a);
        0
    }

    #[inline]
    fn execute_tsx(&mut self) -> u32 {
        self.x = self.sp;
        self.set_zn_flags(self.x);
        0
    }

    #[inline]
    fn execute_txs(&mut self) -> u32 {
        self.sp = self.x;
        0
    }

    #[inline]
    fn execute_pha(&mut self) -> u32 {
        self.push_word(self.a);
        0
    }

    #[inline]
    fn execute_php(&mut self) -> u32 {
        self.push_word(Wrapping(
            (self.status | StatusFlags::B | StatusFlags::U).bits(),
        ));
        self.status.remove(StatusFlags::B | StatusFlags::U);
        0
    }

    #[inline]
    fn execute_pla(&mut self) -> u32 {
        self.a = self.pop_word();
        self.set_zn_flags(self.a);
        0
    }

    #[inline]
    fn execute_plp(&mut self) -> u32 {
        unsafe {
            self.status = StatusFlags::from_bits_unchecked(self.pop_word().0);
        }
        self.status.insert(StatusFlags::U);
        0
    }

    #[inline]
    fn execute_and(&mut self, data: ExecutionData) -> u32 {
        self.a &= data.read_data(self);
        self.set_zn_flags(self.a);
        0
    }

    #[inline]
    fn execute_eor(&mut self, data: ExecutionData) -> u32 {
        self.a ^= data.read_data(self);
        self.set_zn_flags(self.a);
        0
    }

    #[inline]
    fn execute_ora(&mut self, data: ExecutionData) -> u32 {
        self.a |= data.read_data(self);
        self.set_zn_flags(self.a);
        0
    }

    fn execute_bit(&mut self, data: ExecutionData) -> u32 {
        let value = data.read_data(self);
        self.status.set(StatusFlags::Z, (self.a & value).0 == 0);
        self.status.set(StatusFlags::N, (value.0 & 0x80) != 0);
        self.status.set(StatusFlags::V, (value.0 & 0x40) != 0);
        0
    }

    fn execute_adc_decimal(&mut self, right: u16) -> u32 {
        let left = self.a.0 as u16;
        let carry: u16 = if self.status.contains(StatusFlags::C) {
            1
        } else {
            0
        };

        let ld0 = left & 0x0F;
        let rd0 = right & 0x0F;
        let ld1 = left & 0xF0;
        let rd1 = right & 0xF0;

        let mut result = ld0 + rd0 + carry;
        if result >= 0x000A {
            result += 0x0006
        }
        result += ld1 + rd1;
        let invalid_n = (result & 0x0080) != 0;
        let is_overflow = ((!(left ^ right) & (left ^ result)) & 0x0080) != 0;
        if result >= 0x00A0 {
            result += 0x0060
        }

        self.a = Wrapping((result & 0x00FF) as u8);
        self.status.set(StatusFlags::C, result >= 0x0100);
        self.status.set(StatusFlags::V, is_overflow);

        if self.emulate_invalid_decimal_flags {
            self.status
                .set(StatusFlags::Z, ((left + right + carry) & 0x00FF) == 0);
            self.status.set(StatusFlags::N, invalid_n);
            0
        } else {
            self.set_zn_flags(self.a);
            1
        }
    }

    fn execute_sbc_decimal(&mut self, right: u16) -> u32 {
        let left = self.a.0 as u16;
        let carry: i16 = if self.status.contains(StatusFlags::C) {
            1
        } else {
            0
        };

        let ld0 = (left & 0x0F) as i16;
        let rd0 = (right & 0x0F) as i16;
        let ld1 = (left & 0xF0) as i16;
        let rd1 = (right & 0xF0) as i16;

        let mut result = ld0 - rd0 + carry - 1;
        if result < 0 {
            result = ((result - 0x0006) & 0x000F) - 0x0010
        }
        result = ld1 - rd1 + result;
        if result < 0 {
            result -= 0x0060
        }

        self.a = Wrapping((result & 0x00FF) as u8);

        let bin_result = left + right + (carry as u16);
        let is_overflow = ((!(left ^ right) & (left ^ bin_result)) & 0x0080) != 0;
        self.status.set(StatusFlags::C, (bin_result & 0xFF00) != 0);
        self.status.set(StatusFlags::V, is_overflow);

        if self.emulate_invalid_decimal_flags {
            self.set_zn_flags(Wrapping((bin_result & 0x00FF) as u8));
            0
        } else {
            self.set_zn_flags(self.a);
            1
        }
    }

    fn execute_adc_sbc(&mut self, right: u16) -> u32 {
        let left = self.a.0 as u16;
        let carry: u16 = if self.status.contains(StatusFlags::C) {
            1
        } else {
            0
        };

        let result = left + right + carry;
        let is_overflow = ((!(left ^ right) & (left ^ result)) & 0x0080) != 0;

        self.a = Wrapping((result & 0x00FF) as u8);
        self.status.set(StatusFlags::C, (result & 0xFF00) != 0);
        self.status.set(StatusFlags::V, is_overflow);
        self.set_zn_flags(self.a);

        0
    }

    fn execute_adc(&mut self, data: ExecutionData) -> u32 {
        let right = data.read_data(self).0 as u16;
        if self.enable_decimal_mode && self.status.contains(StatusFlags::D) {
            self.execute_adc_decimal(right)
        } else {
            self.execute_adc_sbc(right)
        }
    }

    fn execute_sbc(&mut self, data: ExecutionData) -> u32 {
        if self.enable_decimal_mode && self.status.contains(StatusFlags::D) {
            let right = data.read_data(self).0 as u16;
            self.execute_sbc_decimal(right)
        } else {
            let right = (!data.read_data(self).0) as u16;
            self.execute_adc_sbc(right)
        }
    }

    fn execute_cmp(&mut self, data: ExecutionData) -> u32 {
        let value = data.read_data(self);
        let tmp = self.a - value;
        self.status.set(StatusFlags::C, self.a >= value);
        self.set_zn_flags(tmp);
        0
    }

    fn execute_cpx(&mut self, data: ExecutionData) -> u32 {
        let value = data.read_data(self);
        let tmp = self.x - value;
        self.status.set(StatusFlags::C, self.x >= value);
        self.set_zn_flags(tmp);
        0
    }

    fn execute_cpy(&mut self, data: ExecutionData) -> u32 {
        let value = data.read_data(self);
        let tmp = self.y - value;
        self.status.set(StatusFlags::C, self.y >= value);
        self.set_zn_flags(tmp);
        0
    }

    #[inline]
    fn execute_inc(&mut self, data: ExecutionData) -> u32 {
        if let ExecutionData::None = data {
            // If no address is provided the operation is applied to the accumulator
            self.a += Wrapping(1);
            self.set_zn_flags(self.a);
        } else {
            let value = data.read_data(self) + Wrapping(1);
            data.write_data(self, value);
            self.set_zn_flags(value);
        }

        0
    }

    #[inline]
    fn execute_inx(&mut self) -> u32 {
        self.x += Wrapping(1);
        self.set_zn_flags(self.x);
        0
    }

    #[inline]
    fn execute_iny(&mut self) -> u32 {
        self.y += Wrapping(1);
        self.set_zn_flags(self.y);
        0
    }

    #[inline]
    fn execute_dec(&mut self, data: ExecutionData) -> u32 {
        if let ExecutionData::None = data {
            // If no address is provided the operation is applied to the accumulator
            self.a -= Wrapping(1);
            self.set_zn_flags(self.a);
        } else {
            let value = data.read_data(self) - Wrapping(1);
            data.write_data(self, value);
            self.set_zn_flags(value);
        }

        0
    }

    #[inline]
    fn execute_dex(&mut self) -> u32 {
        self.x -= Wrapping(1);
        self.set_zn_flags(self.x);
        0
    }

    #[inline]
    fn execute_dey(&mut self) -> u32 {
        self.y -= Wrapping(1);
        self.set_zn_flags(self.y);
        0
    }

    fn execute_asl(&mut self, data: ExecutionData) -> u32 {
        if let ExecutionData::None = data {
            // If no address is provided the operation is applied to the accumulator
            self.status.set(StatusFlags::C, (self.a.0 & 0x80) != 0);
            self.a <<= 1;
            self.set_zn_flags(self.a);
        } else {
            let value = data.read_data(self);
            self.status.set(StatusFlags::C, (value.0 & 0x80) != 0);

            let tmp = value << 1;
            self.set_zn_flags(tmp);
            data.write_data(self, tmp);
        }

        0
    }

    fn execute_lsr(&mut self, data: ExecutionData) -> u32 {
        if let ExecutionData::None = data {
            // If no address is provided the operation is applied to the accumulator
            self.status.set(StatusFlags::C, (self.a.0 & 0x01) != 0);
            self.a >>= 1;
            self.set_zn_flags(self.a);
        } else {
            let value = data.read_data(self);
            self.status.set(StatusFlags::C, (value.0 & 0x01) != 0);

            let tmp = value >> 1;
            self.set_zn_flags(tmp);
            data.write_data(self, tmp);
        }

        0
    }

    fn execute_rol(&mut self, data: ExecutionData) -> u32 {
        if let ExecutionData::None = data {
            // If no address is provided the operation is applied to the accumulator
            let tmp = ((self.a.0 as u16) << 1)
                | if self.status.contains(StatusFlags::C) {
                    0x0001
                } else {
                    0x0000
                };
            self.status.set(StatusFlags::C, (tmp & 0xFF00) != 0);
            self.a = Wrapping((tmp & 0x00FF) as u8);
            self.set_zn_flags(self.a);
        } else {
            let value = data.read_data(self);
            let tmp = ((value.0 as u16) << 1)
                | if self.status.contains(StatusFlags::C) {
                    0x0001
                } else {
                    0x0000
                };
            self.status.set(StatusFlags::C, (tmp & 0xFF00) != 0);

            let new_value = Wrapping((tmp & 0x00FF) as u8);
            self.set_zn_flags(new_value);
            data.write_data(self, new_value);
        }

        0
    }

    fn execute_ror(&mut self, data: ExecutionData) -> u32 {
        if let ExecutionData::None = data {
            // If no address is provided the operation is applied to the accumulator
            let tmp = (self.a >> 1)
                | if self.status.contains(StatusFlags::C) {
                    Wrapping(0x80)
                } else {
                    Wrapping(0x00)
                };
            self.status.set(StatusFlags::C, (self.a.0 & 0x01) != 0);
            self.a = tmp;
            self.set_zn_flags(self.a);
        } else {
            let value = data.read_data(self);
            let tmp = (value >> 1)
                | if self.status.contains(StatusFlags::C) {
                    Wrapping(0x80)
                } else {
                    Wrapping(0x00)
                };
            self.status.set(StatusFlags::C, (value.0 & 0x01) != 0);
            data.write_data(self, tmp);
            self.set_zn_flags(tmp);
        }

        0
    }

    #[inline]
    fn execute_jmp(&mut self, data: ExecutionData) -> u32 {
        self.pc = data.read_address();
        0
    }

    #[inline]
    fn execute_jsr(&mut self, data: ExecutionData) -> u32 {
        self.pc -= Wrapping(1);
        self.push_address(self.pc);
        self.pc = data.read_address();
        0
    }

    #[inline]
    fn execute_rts(&mut self, _: ExecutionData) -> u32 {
        self.pc = self.pop_address() + Wrapping(1);
        0
    }

    #[inline]
    fn execute_bcc(&mut self, data: ExecutionData) -> u32 {
        if !self.status.contains(StatusFlags::C) {
            self.pc = data.read_address();
            1
        } else {
            0
        }
    }

    #[inline]
    fn execute_bcs(&mut self, data: ExecutionData) -> u32 {
        if self.status.contains(StatusFlags::C) {
            self.pc = data.read_address();
            1
        } else {
            0
        }
    }

    #[inline]
    fn execute_beq(&mut self, data: ExecutionData) -> u32 {
        if self.status.contains(StatusFlags::Z) {
            self.pc = data.read_address();
            1
        } else {
            0
        }
    }

    #[inline]
    fn execute_bmi(&mut self, data: ExecutionData) -> u32 {
        if self.status.contains(StatusFlags::N) {
            self.pc = data.read_address();
            1
        } else {
            0
        }
    }

    #[inline]
    fn execute_bne(&mut self, data: ExecutionData) -> u32 {
        if !self.status.contains(StatusFlags::Z) {
            self.pc = data.read_address();
            1
        } else {
            0
        }
    }

    #[inline]
    fn execute_bpl(&mut self, data: ExecutionData) -> u32 {
        if !self.status.contains(StatusFlags::N) {
            self.pc = data.read_address();
            1
        } else {
            0
        }
    }

    #[inline]
    fn execute_bvc(&mut self, data: ExecutionData) -> u32 {
        if !self.status.contains(StatusFlags::V) {
            self.pc = data.read_address();
            1
        } else {
            0
        }
    }

    #[inline]
    fn execute_bvs(&mut self, data: ExecutionData) -> u32 {
        if self.status.contains(StatusFlags::V) {
            self.pc = data.read_address();
            1
        } else {
            0
        }
    }

    #[inline]
    fn execute_clc(&mut self) -> u32 {
        self.status.remove(StatusFlags::C);
        0
    }

    #[inline]
    fn execute_cld(&mut self) -> u32 {
        self.status.remove(StatusFlags::D);
        0
    }

    #[inline]
    fn execute_cli(&mut self) -> u32 {
        self.status.remove(StatusFlags::I);
        0
    }

    #[inline]
    fn execute_clv(&mut self) -> u32 {
        self.status.remove(StatusFlags::V);
        0
    }

    #[inline]
    fn execute_sec(&mut self) -> u32 {
        self.status.insert(StatusFlags::C);
        0
    }

    #[inline]
    fn execute_sed(&mut self) -> u32 {
        self.status.insert(StatusFlags::D);
        0
    }

    #[inline]
    fn execute_sei(&mut self) -> u32 {
        self.status.insert(StatusFlags::I);
        0
    }

    #[inline]
    fn execute_brk(&mut self) -> u32 {
        self.pc += Wrapping(1);
        self.push_address(self.pc);

        self.status.insert(StatusFlags::B | StatusFlags::I);
        self.push_word(Wrapping(self.status.bits()));
        self.status.remove(StatusFlags::B);

        self.pc = self.read_address(IRQ_VECTOR);
        0
    }

    #[inline]
    fn execute_rti(&mut self) -> u32 {
        unsafe {
            self.status = StatusFlags::from_bits_unchecked(self.pop_word().0);
        }
        self.status.remove(StatusFlags::B | StatusFlags::U);
        self.pc = self.pop_address();
        0
    }

    fn execute_slo(&mut self, data: ExecutionData) -> u32 {
        let value = data.read_data(self);
        self.status.set(StatusFlags::C, (value.0 & 0x80) != 0);

        let tmp = value << 1;
        data.write_data(self, tmp);

        self.a |= tmp;
        self.set_zn_flags(self.a);

        0
    }

    #[inline]
    fn execute_anc(&mut self, data: ExecutionData) -> u32 {
        self.status.set(StatusFlags::C, (self.a.0 & 0x80) != 0);
        self.a &= data.read_data(self);
        self.set_zn_flags(self.a);
        0
    }

    fn execute_rla(&mut self, data: ExecutionData) -> u32 {
        let value = data.read_data(self);
        let tmp = ((value.0 as u16) << 1)
            | if self.status.contains(StatusFlags::C) {
                0x0001
            } else {
                0x0000
            };
        self.status.set(StatusFlags::C, (tmp & 0xFF00) != 0);

        let new_value = Wrapping((tmp & 0x00FF) as u8);
        data.write_data(self, new_value);

        self.a &= new_value;
        self.set_zn_flags(self.a);

        0
    }

    fn execute_sre(&mut self, data: ExecutionData) -> u32 {
        let value = data.read_data(self);
        self.status.set(StatusFlags::C, (value.0 & 0x01) != 0);

        let tmp = value >> 1;
        data.write_data(self, tmp);

        self.a ^= tmp;
        self.set_zn_flags(self.a);

        0
    }

    #[inline]
    fn execute_alr(&mut self, data: ExecutionData) -> u32 {
        self.a &= data.read_data(self);
        self.status.set(StatusFlags::C, (self.a.0 & 0x01) != 0);
        self.a >>= 1;
        self.set_zn_flags(self.a);
        0
    }

    fn execute_rra(&mut self, data: ExecutionData) -> u32 {
        let value = data.read_data(self);
        let tmp = (value >> 1)
            | if self.status.contains(StatusFlags::C) {
                Wrapping(0x80)
            } else {
                Wrapping(0x00)
            };
        self.status.set(StatusFlags::C, (value.0 & 0x01) != 0);
        data.write_data(self, tmp);

        let right = tmp.0 as u16;
        self.execute_adc_sbc(right)
    }

    fn execute_arr(&mut self, data: ExecutionData) -> u32 {
        self.a &= data.read_data(self);
        let tmp = (self.a >> 1)
            | if self.status.contains(StatusFlags::C) {
                Wrapping(0x80)
            } else {
                Wrapping(0x00)
            };
        self.a = tmp;
        self.set_zn_flags(self.a);
        0
    }

    #[inline]
    fn execute_sax(&mut self, data: ExecutionData) -> u32 {
        data.write_data(self, self.a & self.x);
        0
    }

    #[inline]
    fn execute_xaa(&mut self, data: ExecutionData) -> u32 {
        self.a = self.a & self.x & data.read_data(self);
        self.set_zn_flags(self.a);
        0
    }

    #[inline]
    fn execute_ahx(&mut self, data: ExecutionData) -> u32 {
        data.write_data(self, self.a & self.x & data.read_data(self));
        0
    }

    #[inline]
    fn execute_tas(&mut self, data: ExecutionData) -> u32 {
        self.sp = self.a & self.x;
        data.write_data(self, self.a & self.x & data.read_data(self));
        0
    }

    #[inline]
    fn execute_shy(&mut self, data: ExecutionData) -> u32 {
        data.write_data(self, self.y & data.read_data(self));
        0
    }

    #[inline]
    fn execute_shx(&mut self, data: ExecutionData) -> u32 {
        data.write_data(self, self.x & data.read_data(self));
        0
    }

    #[inline]
    fn execute_lax(&mut self, data: ExecutionData) -> u32 {
        self.a = data.read_data(self);
        self.x = self.a;
        self.set_zn_flags(self.a);
        0
    }

    #[inline]
    fn execute_las(&mut self, data: ExecutionData) -> u32 {
        self.a = data.read_data(self) & self.sp;
        self.x = self.a;
        self.sp = self.a;
        self.set_zn_flags(self.a);
        0
    }

    fn execute_dcp(&mut self, data: ExecutionData) -> u32 {
        let value = data.read_data(self) - Wrapping(1);
        data.write_data(self, value);

        let tmp = self.a - value;
        self.status.set(StatusFlags::C, self.a >= value);
        self.set_zn_flags(tmp);

        0
    }

    #[inline]
    fn execute_axs(&mut self, data: ExecutionData) -> u32 {
        let value = data.read_data(self);
        let tmp = (self.a & self.x) - value;
        self.status.set(StatusFlags::C, (self.a & self.x) >= value);
        self.set_zn_flags(tmp);
        self.x = tmp;
        0
    }

    #[inline]
    fn execute_isc(&mut self, data: ExecutionData) -> u32 {
        let value = data.read_data(self) + Wrapping(1);
        data.write_data(self, value);

        let right = (!value.0) as u16;
        self.execute_adc_sbc(right)
    }

    /*
        65C02 instructions
    */

    #[inline]
    fn execute_bra(&mut self, data: ExecutionData) -> u32 {
        self.pc = data.read_address();
        1
    }

    #[inline]
    fn execute_phx(&mut self) -> u32 {
        self.push_word(self.x);
        0
    }

    #[inline]
    fn execute_phy(&mut self) -> u32 {
        self.push_word(self.y);
        0
    }

    #[inline]
    fn execute_plx(&mut self) -> u32 {
        self.x = self.pop_word();
        self.set_zn_flags(self.x);
        0
    }

    #[inline]
    fn execute_ply(&mut self) -> u32 {
        self.y = self.pop_word();
        self.set_zn_flags(self.y);
        0
    }

    #[inline]
    fn execute_stz(&mut self, data: ExecutionData) -> u32 {
        data.write_data(self, Wrapping(0));
        0
    }

    #[inline]
    fn execute_trb(&mut self, data: ExecutionData) -> u32 {
        let mut value = data.read_data(self).0;
        value &= !self.a.0;
        self.status.set(StatusFlags::Z, value == 0);
        data.write_data(self, Wrapping(value));

        0
    }

    #[inline]
    fn execute_tsb(&mut self, data: ExecutionData) -> u32 {
        let mut value = data.read_data(self).0;
        value |= self.a.0;
        self.status.set(StatusFlags::Z, value == 0);
        data.write_data(self, Wrapping(value));

        0
    }

    #[inline]
    fn execute_bbr(&mut self, data: ExecutionData, n: usize) -> u32 {
        let value = data.read_data(self).0;

        if (value & (0x01 << n)) == 0 {
            self.pc = data.read_address();
            1
        } else {
            0
        }
    }

    #[inline]
    fn execute_bbs(&mut self, data: ExecutionData, n: usize) -> u32 {
        let value = data.read_data(self).0;

        if (value & (0x01 << n)) != 0 {
            self.pc = data.read_address();
            1
        } else {
            0
        }
    }

    #[inline]
    fn execute_rmb(&mut self, data: ExecutionData, n: usize) -> u32 {
        let mut value = data.read_data(self).0;
        value &= !(0x01 << n);
        self.status.set(StatusFlags::Z, value == 0);
        data.write_data(self, Wrapping(value));

        0
    }

    #[inline]
    fn execute_smb(&mut self, data: ExecutionData, n: usize) -> u32 {
        let mut value = data.read_data(self).0;
        value |= 0x01 << n;
        self.status.set(StatusFlags::Z, value == 0);
        data.write_data(self, Wrapping(value));

        0
    }
}
