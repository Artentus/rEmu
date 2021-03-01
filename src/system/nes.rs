use crate::audio::apu2A03::{Apu2A03, Apu2A03Control};
use crate::audio::*;
use crate::bus::*;
use crate::cpu::cpu6502::Cpu6502;
use crate::cpu::*;
use crate::memory::Ram;
use crate::util::BinReader;
use crate::video::ppu2C02::Ppu2C02;
use crate::video::*;
use crate::*;
use std::cell::Ref;
use std::path::Path;

pub const NES_BASE_CLOCK: u32 = 21477272; // 21.47727 MHz
pub const NES_CPU_CLOCK: u32 = NES_BASE_CLOCK / 12;
pub const NES_PPU_CLOCK: u32 = NES_BASE_CLOCK / 4;
pub const NES_APU_CLOCK: u32 = NES_CPU_CLOCK / 2;

#[allow(dead_code)]
pub struct Nes<'a> {
    cpu: Cpu6502<'a>,
    cpu_bus: EmuRef<Bus<'a, cpu6502::Address, cpu6502::Word>>,
    ram: EmuRef<Ram<cpu6502::Address, cpu6502::Word>>,
    apu: EmuRef<Apu2A03<'a>>,
    apu_control: EmuRef<Apu2A03Control<'a>>,
    dma: EmuRef<DmaInterface>,
    controller: EmuRef<VController>,

    ppu: EmuRef<Ppu2C02<'a>>,
    ppu_bus: EmuRef<Bus<'a, ppu2C02::Address, ppu2C02::Word>>,
    vram: EmuRef<Vram>,
    palette: EmuRef<Ram<ppu2C02::Address, ppu2C02::Word>>,

    cartridge: Option<EmuRef<Cartridge>>,
    cartridge_cpu_handle: Option<BusHandle>,
    cartridge_ppu_handle: Option<BusHandle>,

    cycle_even: bool,
}
impl<'a> Nes<'a> {
    pub fn new() -> Self {
        /* PPU bus */
        const VRAM_START_ADDRESS: ppu2C02::Address = Wrapping(0x2000);
        const VRAM_MIRRORED_END_ADDRESS: ppu2C02::Address = Wrapping(0x3EFF);
        const PALETTE_SIZE: ppu2C02::Address = Wrapping(0x0020);
        const PALETTE_START_ADDRESS: ppu2C02::Address = Wrapping(0x3F00);
        const PALETTE_MIRRORED_END_ADDRESS: ppu2C02::Address = Wrapping(0x3FFF);

        let vram = Vram::create(VRAM_START_ADDRESS);
        let vram_clone = clone_ref(&vram);
        let mirrored_vram = mirror_component(vram_clone, VRAM_MIRRORED_END_ADDRESS);

        let palette = Ram::create(PALETTE_SIZE, PALETTE_START_ADDRESS);
        let palette_clone = clone_ref(&palette);
        let mirrored_palette = mirror_component(palette_clone, PALETTE_MIRRORED_END_ADDRESS);

        let ppu_bus = Bus::create();
        {
            let mut ppu_bus_borrow = ppu_bus.borrow_mut();
            ppu_bus_borrow.add_component(mirrored_vram);
            ppu_bus_borrow.add_component(mirrored_palette);
        }
        /* End PPU bus */

        /* CPU bus */
        const RAM_START_ADDRESS: cpu6502::Address = Wrapping(0);
        const RAM_SIZE: cpu6502::Address = Wrapping(0x0800);
        const RAM_MIRRORED_END_ADDRESS: cpu6502::Address = Wrapping(0x1FFF);
        const PPU_START_ADDRESS: cpu6502::Address = Wrapping(0x2000);
        const PPU_MIRRORED_END_ADDRESS: cpu6502::Address = Wrapping(0x3FFF);
        const APU_START_ADDRESS: cpu6502::Address = Wrapping(0x4000);
        const APU_CONTROLL_ADDRESS: cpu6502::Address = Wrapping(0x4015);
        const DMA_ADDRESS: cpu6502::Address = Wrapping(0x4014);
        const CONTROLLER_START_ADDRESS: cpu6502::Address = Wrapping(0x4016);

        let cpu_bus = Bus::create();

        let ram = Ram::create(RAM_SIZE, RAM_START_ADDRESS);
        let ram_clone = clone_ref(&ram);
        let mirrored_ram = mirror_component(ram_clone, RAM_MIRRORED_END_ADDRESS);

        let ppu = Ppu2C02::create(clone_ref(&ppu_bus), PPU_START_ADDRESS);
        let ppu_clone = clone_ref(&ppu);
        let mirrored_ppu = mirror_component(ppu_clone, PPU_MIRRORED_END_ADDRESS);

        let apu = Apu2A03::create(APU_START_ADDRESS, clone_ref(&cpu_bus));
        let apu_clone = clone_ref(&apu);
        let apu_control = Apu2A03Control::create(APU_CONTROLL_ADDRESS, clone_ref(&apu));
        let apu_control_clone = clone_ref(&apu_control);

        let dma = DmaInterface::create(DMA_ADDRESS);
        let dma_clone = clone_ref(&dma);

        let controller = VController::create(CONTROLLER_START_ADDRESS);
        let controller_clone = clone_ref(&controller);

        {
            let mut cpu_bus_borrow = cpu_bus.borrow_mut();
            cpu_bus_borrow.add_component(mirrored_ram);
            cpu_bus_borrow.add_component(mirrored_ppu);
            cpu_bus_borrow.add_component(apu_clone);
            cpu_bus_borrow.add_component(apu_control_clone);
            cpu_bus_borrow.add_component(dma_clone);
            cpu_bus_borrow.add_component(controller_clone);
        }
        /* End CPU bus */

        let cpu = Cpu6502::new(clone_ref(&cpu_bus));

        Self {
            cpu,
            cpu_bus,
            ram,
            apu,
            apu_control,
            dma,
            controller,
            ppu,
            ppu_bus,
            vram,
            palette,
            cartridge: None,
            cartridge_cpu_handle: None,
            cartridge_ppu_handle: None,
            cycle_even: true,
        }
    }

    pub fn set_cartridge(&mut self, cartridge: EmuRef<Cartridge>) {
        {
            let cartridge_borrow = cartridge.borrow();
            self.cartridge_cpu_handle = Some(
                self.cpu_bus
                    .borrow_mut()
                    .add_component(cartridge_borrow.get_cpu_adapter()),
            );
            self.cartridge_ppu_handle = Some(
                self.ppu_bus
                    .borrow_mut()
                    .add_component(cartridge_borrow.get_ppu_adapter()),
            );
        }
        self.vram.borrow_mut().set_cartridge(clone_ref(&cartridge));
        self.ppu.borrow_mut().set_cartridge(clone_ref(&cartridge));
        self.cartridge = Some(cartridge);
    }

    pub fn remove_cartridge(&mut self) {
        if let Some(handle) = self.cartridge_cpu_handle {
            self.cpu_bus.borrow_mut().remove_component(handle);
        }
        if let Some(handle) = self.cartridge_ppu_handle {
            self.ppu_bus.borrow_mut().remove_component(handle);
        }
        self.vram.borrow_mut().remove_cartridge();
        self.ppu.borrow_mut().remove_cartridge();

        self.cartridge = None;
        self.cartridge_cpu_handle = None;
        self.cartridge_ppu_handle = None;
    }

    pub fn reset(&mut self) {
        self.cpu.reset();
        self.ppu.borrow_mut().reset();
        self.apu.borrow_mut().reset();
        if let Some(cartridge_ref) = &self.cartridge {
            let mut cartridge = cartridge_ref.borrow_mut();
            cartridge.reset_interrupt();
            cartridge.reset_mapper();
        }
    }

    #[inline]
    pub fn screen(&self) -> Ref<dyn VideoBuffer> {
        Ref::map(self.ppu.borrow(), |ppu| ppu.get_buffer())
    }

    #[inline]
    pub fn update_input_state(&mut self, controller_0: Buttons, controller_1: Buttons) {
        self.controller
            .borrow_mut()
            .update_state(controller_0, controller_1);
    }

    fn next_instruction(&mut self, buffer: &mut SampleBuffer) {
        let nmi = { self.ppu.borrow_mut().check_nmi() };

        let irq = if let Some(cartridge) = &self.cartridge {
            let mut cart = cartridge.borrow_mut();
            if cart.interrupt_state() {
                cart.reset_interrupt();
                true
            } else {
                false
            }
        } else {
            false
        };

        let mut dma = self.dma.borrow_mut();
        let cpu_cycles = if dma.active {
            dma.active = false;
            let address = (dma.page.0 as u16) << 8;
            std::mem::drop(dma);

            let cpu_bus_borrow = self.cpu_bus.borrow();
            let mut ppu_borrow = self.ppu.borrow_mut();
            for i in 0..256u16 {
                let data = cpu_bus_borrow.read(Wrapping(address | i));
                ppu_borrow.dma_write(Wrapping(i as u8), data);
            }

            512 + if self.cycle_even { 0 } else { 1 }
        } else {
            std::mem::drop(dma);

            if nmi {
                self.cpu.nmi()
            } else if irq {
                self.cpu.irq()
            } else {
                self.cpu.execute_next_instruction()
            }
        };

        self.cycle_even = self.cycle_even & ((cpu_cycles % 2) == 0);

        self.apu.borrow_mut().clock(cpu_cycles, buffer);

        let ppu_cycles = cpu_cycles * 3;
        self.ppu.borrow_mut().clock(ppu_cycles);
    }

    #[inline]
    pub fn next_frame(&mut self, buffer: &mut SampleBuffer) {
        let buffer_length_before = buffer.len();
        while (buffer.len() - buffer_length_before) < ((SAMPLE_RATE / FRAME_RATE) as usize) {
            self.next_instruction(buffer);
        }
    }
}

const PRG_BANK_SIZE: usize = 0x4000;
const CHR_BANK_SIZE: usize = 0x2000;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum MirrorMode {
    Horizontal,
    Vertical,
    OneScreenLow,
    OneScreenHigh,
}

enum MapperReadResult {
    Data(cpu6502::Word),
    Address(Option<usize>),
}

trait Mapper {
    fn mirror(&self) -> Option<MirrorMode>;

    fn interrupt_state(&self) -> bool;

    fn reset_interrupt(&mut self);

    fn on_scanline(&mut self);

    fn cpu_read(&self, addr: cpu6502::Address) -> MapperReadResult;

    fn ppu_read(&self, addr: ppu2C02::Address) -> MapperReadResult;

    fn cpu_write(&mut self, addr: cpu6502::Address, data: cpu6502::Word);

    fn reset(&mut self);
}

struct NRom {
    mask: u16,
}
impl NRom {
    fn new(prg_banks: u8) -> Self {
        Self {
            mask: if prg_banks > 1 { 0x7FFF } else { 0x3FFF },
        }
    }
}
impl Mapper for NRom {
    fn mirror(&self) -> Option<MirrorMode> {
        None
    }

    fn interrupt_state(&self) -> bool {
        false
    }

    fn reset_interrupt(&mut self) {}

    fn on_scanline(&mut self) {}

    fn cpu_read(&self, addr: cpu6502::Address) -> MapperReadResult {
        if addr.0 >= 0x8000 {
            MapperReadResult::Address(Some((addr.0 & self.mask) as usize))
        } else {
            MapperReadResult::Address(None)
        }
    }

    fn ppu_read(&self, addr: ppu2C02::Address) -> MapperReadResult {
        if addr.0 <= 0x1FFF {
            MapperReadResult::Address(Some(addr.0 as usize))
        } else {
            MapperReadResult::Address(None)
        }
    }

    fn cpu_write(&mut self, _addr: cpu6502::Address, _data: cpu6502::Word) {}

    fn reset(&mut self) {}
}

struct Mmc1 {
    prg_banks: u8,
    load: u8,
    load_count: u8,
    control: u8,
    prg_bank_32: u8,
    chr_bank_8: u8,
    prg_bank_16_lo: u8,
    prg_bank_16_hi: u8,
    chr_bank_4_lo: u8,
    chr_bank_4_hi: u8,
    mirror: MirrorMode,
    prg_ram: Box<[Wrapping<u8>]>,
}
impl Mmc1 {
    fn new(prg_banks: u8) -> Self {
        Self {
            prg_banks,
            load: 0,
            load_count: 0,
            control: 0x1C,
            prg_bank_32: 0,
            chr_bank_8: 0,
            prg_bank_16_lo: 0,
            prg_bank_16_hi: prg_banks - 1,
            chr_bank_4_lo: 0,
            chr_bank_4_hi: 0,
            mirror: MirrorMode::Horizontal,
            prg_ram: vec![Wrapping(0); 0x2000].into_boxed_slice(),
        }
    }
}
impl Mapper for Mmc1 {
    fn mirror(&self) -> Option<MirrorMode> {
        Some(self.mirror)
    }

    fn interrupt_state(&self) -> bool {
        false
    }

    fn reset_interrupt(&mut self) {}

    fn on_scanline(&mut self) {}

    fn cpu_read(&self, addr: cpu6502::Address) -> MapperReadResult {
        if (addr.0 >= 0x6000) && (addr.0 <= 0x7FFF) {
            MapperReadResult::Data(self.prg_ram[(addr.0 & 0x1FFF) as usize])
        } else if addr.0 >= 0x8000 {
            if (self.control & 0x08) != 0 {
                // 16k mode
                if addr.0 <= 0xBFFF {
                    MapperReadResult::Address(Some(
                        (self.prg_bank_16_lo as usize) * PRG_BANK_SIZE
                            + ((addr.0 & 0x3FFF) as usize),
                    ))
                } else {
                    MapperReadResult::Address(Some(
                        (self.prg_bank_16_hi as usize) * PRG_BANK_SIZE
                            + ((addr.0 & 0x3FFF) as usize),
                    ))
                }
            } else {
                // 32k mode
                MapperReadResult::Address(Some(
                    (self.prg_bank_32 as usize) * 2 * PRG_BANK_SIZE + ((addr.0 & 0x7FFF) as usize),
                ))
            }
        } else {
            MapperReadResult::Address(None)
        }
    }

    fn ppu_read(&self, addr: ppu2C02::Address) -> MapperReadResult {
        if addr.0 <= 0x1FFF {
            if (self.control & 0x10) != 0 {
                // 4k mode
                if addr.0 <= 0x0FFF {
                    MapperReadResult::Address(Some(
                        (self.chr_bank_4_lo as usize) * 0x1000 + ((addr.0 & 0x0FFF) as usize),
                    ))
                } else {
                    MapperReadResult::Address(Some(
                        (self.chr_bank_4_hi as usize) * 0x1000 + ((addr.0 & 0x0FFF) as usize),
                    ))
                }
            } else {
                // 8k mode
                MapperReadResult::Address(Some(
                    (self.chr_bank_8 as usize) * CHR_BANK_SIZE + ((addr.0 & 0x1FFF) as usize),
                ))
            }
        } else {
            MapperReadResult::Address(None)
        }
    }

    fn cpu_write(&mut self, addr: cpu6502::Address, data: cpu6502::Word) {
        if (addr.0 >= 0x6000) && (addr.0 <= 0x7FFF) {
            self.prg_ram[(addr.0 & 0x1FFF) as usize] = data;
        } else if addr.0 >= 0x8000 {
            if (data.0 & 0x80) != 0 {
                self.load = 0;
                self.load_count = 0;
                self.control |= 0x0C;
            } else {
                self.load >>= 1;
                self.load |= (data.0 & 0x01) << 4;
                self.load_count += 1;

                if self.load_count == 5 {
                    let target_reg = (addr.0 >> 13) & 0x03;

                    match target_reg {
                        0 => {
                            // Control register
                            self.control = self.load & 0x1F;
                            self.mirror = match self.control & 0x03 {
                                0 => MirrorMode::OneScreenLow,
                                1 => MirrorMode::OneScreenHigh,
                                2 => MirrorMode::Vertical,
                                3 => MirrorMode::Horizontal,
                                _ => unreachable!(),
                            }
                        }
                        1 => {
                            // CHR low bank
                            if (self.control & 0x10) != 0 {
                                self.chr_bank_4_lo = self.load & 0x1F;
                            } else {
                                self.chr_bank_8 = self.load & 0x1E;
                            }
                        }
                        2 => {
                            // CHR high bank
                            if (self.control & 0x10) != 0 {
                                self.chr_bank_4_hi = self.load & 0x1F;
                            }
                        }
                        3 => {
                            // PRG banks
                            let prg_mode = (self.control >> 2) & 0x03;

                            if prg_mode <= 1 {
                                self.prg_bank_32 = (self.load & 0x0E) >> 1;
                            } else if prg_mode == 2 {
                                self.prg_bank_16_lo = 0;
                                self.prg_bank_16_hi = self.load & 0x0F;
                            } else if prg_mode == 3 {
                                self.prg_bank_16_lo = self.load & 0x0F;
                                self.prg_bank_16_hi = self.prg_banks - 1;
                            }
                        }
                        _ => unreachable!(),
                    }

                    self.load = 0;
                    self.load_count = 0;
                }
            }
        }
    }

    fn reset(&mut self) {
        self.load = 0;
        self.load_count = 0;
        self.control = 0x1C;
        self.prg_bank_32 = 0;
        self.chr_bank_8 = 0;
        self.prg_bank_16_lo = 0;
        self.prg_bank_16_hi = self.prg_banks - 1;
        self.chr_bank_4_lo = 0;
        self.chr_bank_4_hi = 0;
    }
}

struct UxRom {
    prg_bank_lo: u8,
    prg_bank_hi: u8,
}
impl UxRom {
    fn new(prg_banks: u8) -> Self {
        Self {
            prg_bank_lo: 0,
            prg_bank_hi: prg_banks - 1,
        }
    }
}
impl Mapper for UxRom {
    fn mirror(&self) -> Option<MirrorMode> {
        None
    }

    fn interrupt_state(&self) -> bool {
        false
    }

    fn reset_interrupt(&mut self) {}

    fn on_scanline(&mut self) {}

    fn cpu_read(&self, addr: cpu6502::Address) -> MapperReadResult {
        if (addr.0 >= 0x8000) && (addr.0 <= 0xBFFF) {
            MapperReadResult::Address(Some(
                (self.prg_bank_lo as usize) * PRG_BANK_SIZE + ((addr.0 & 0x3FFF) as usize),
            ))
        } else if addr.0 >= 0xC000 {
            MapperReadResult::Address(Some(
                (self.prg_bank_hi as usize) * PRG_BANK_SIZE + ((addr.0 & 0x3FFF) as usize),
            ))
        } else {
            MapperReadResult::Address(None)
        }
    }

    fn ppu_read(&self, addr: ppu2C02::Address) -> MapperReadResult {
        if addr.0 <= 0x1FFF {
            MapperReadResult::Address(Some(addr.0 as usize))
        } else {
            MapperReadResult::Address(None)
        }
    }

    fn cpu_write(&mut self, addr: cpu6502::Address, data: cpu6502::Word) {
        if addr.0 >= 0x8000 {
            self.prg_bank_lo = data.0 & 0x0F;
        }
    }

    fn reset(&mut self) {
        self.prg_bank_lo = 0;
    }
}

struct CNRom {
    mask: u16,
    chr_bank: u8,
}
impl CNRom {
    fn new(prg_banks: u8) -> Self {
        Self {
            mask: if prg_banks > 1 { 0x7FFF } else { 0x3FFF },
            chr_bank: 0,
        }
    }
}
impl Mapper for CNRom {
    fn mirror(&self) -> Option<MirrorMode> {
        None
    }

    fn interrupt_state(&self) -> bool {
        false
    }

    fn reset_interrupt(&mut self) {}

    fn on_scanline(&mut self) {}

    fn cpu_read(&self, addr: cpu6502::Address) -> MapperReadResult {
        if addr.0 >= 0x8000 {
            MapperReadResult::Address(Some((addr.0 & self.mask) as usize))
        } else {
            MapperReadResult::Address(None)
        }
    }

    fn ppu_read(&self, addr: ppu2C02::Address) -> MapperReadResult {
        if addr.0 <= 0x1FFF {
            MapperReadResult::Address(Some(
                (self.chr_bank as usize) * CHR_BANK_SIZE + (addr.0 as usize),
            ))
        } else {
            MapperReadResult::Address(None)
        }
    }

    fn cpu_write(&mut self, addr: cpu6502::Address, data: cpu6502::Word) {
        if addr.0 >= 0x8000 {
            self.chr_bank = data.0 & 0x03;
        }
    }

    fn reset(&mut self) {
        self.chr_bank = 0;
    }
}

struct Mmc3 {
    target_reg: usize,
    register: [usize; 8],
    prg_bank: [usize; 4],
    chr_bank: [usize; 8],
    interrupt_counter: u16,
    interrupt_step: u16,
    interrupt_active: bool,
    interrupt_enabled: bool,
    prg_bank_mode: bool,
    chr_inversion: bool,
    prg_banks: u8,
    mirror: MirrorMode,
    prg_ram: Box<[Wrapping<u8>]>,
}
impl Mmc3 {
    fn new(prg_banks: u8) -> Self {
        Self {
            target_reg: 0,
            register: [0; 8],
            prg_bank: [
                0,
                0x2000,
                ((prg_banks as usize) * 2 - 2) * 0x2000,
                ((prg_banks as usize) * 2 - 1) * 0x2000,
            ],
            chr_bank: [0; 8],
            interrupt_counter: 0,
            interrupt_step: 0,
            interrupt_active: false,
            interrupt_enabled: false,
            prg_bank_mode: false,
            chr_inversion: false,
            prg_banks,
            mirror: MirrorMode::Horizontal,
            prg_ram: vec![Wrapping(0); 0x2000].into_boxed_slice(),
        }
    }
}
impl Mapper for Mmc3 {
    fn mirror(&self) -> Option<MirrorMode> {
        Some(self.mirror)
    }

    fn interrupt_state(&self) -> bool {
        self.interrupt_active
    }

    fn reset_interrupt(&mut self) {
        self.interrupt_active = false;
    }

    fn on_scanline(&mut self) {
        if self.interrupt_counter == 0 {
            self.interrupt_counter = self.interrupt_step;
        } else {
            self.interrupt_counter -= 1;
        }

        if (self.interrupt_counter == 0) && self.interrupt_enabled {
            self.interrupt_active = true;
        }
    }

    fn cpu_read(&self, addr: cpu6502::Address) -> MapperReadResult {
        if (addr.0 >= 0x6000) && (addr.0 <= 0x7FFF) {
            MapperReadResult::Data(self.prg_ram[(addr.0 & 0x1FFF) as usize])
        } else if addr.0 >= 0x8000 {
            let bank = ((addr.0 >> 13) & 0x03) as usize;
            let mapped_addr = self.prg_bank[bank] + ((addr.0 & 0x1FFF) as usize);
            MapperReadResult::Address(Some(mapped_addr))
        } else {
            MapperReadResult::Address(None)
        }
    }

    fn ppu_read(&self, addr: ppu2C02::Address) -> MapperReadResult {
        if addr.0 <= 0x1FFF {
            let bank = ((addr.0 >> 10) & 0x07) as usize;
            let mapped_addr = self.chr_bank[bank] + ((addr.0 & 0x03FF) as usize);
            MapperReadResult::Address(Some(mapped_addr))
        } else {
            MapperReadResult::Address(None)
        }
    }

    fn cpu_write(&mut self, addr: cpu6502::Address, data: cpu6502::Word) {
        const PRG_BANK_SIZE_L: usize = 0x2000;
        const CHR_BANK_SIZE_L: usize = 0x0400;

        if (addr.0 >= 0x6000) && (addr.0 <= 0x7FFF) {
            self.prg_ram[(addr.0 & 0x1FFF) as usize] = data;
        } else if addr.0 >= 0x8000 {
            if addr.0 <= 0x9FFF {
                // Bank select
                if (addr.0 & 0x0001) == 0 {
                    self.target_reg = (data.0 & 0x07) as usize;
                    self.prg_bank_mode = (data.0 & 0x40) != 0;
                    self.chr_inversion = (data.0 & 0x80) != 0;
                } else {
                    self.register[self.target_reg] = data.0 as usize;

                    if self.chr_inversion {
                        self.chr_bank[0] = self.register[2] * CHR_BANK_SIZE_L;
                        self.chr_bank[1] = self.register[3] * CHR_BANK_SIZE_L;
                        self.chr_bank[2] = self.register[4] * CHR_BANK_SIZE_L;
                        self.chr_bank[3] = self.register[5] * CHR_BANK_SIZE_L;
                        self.chr_bank[4] = (self.register[0] & 0xFE) * CHR_BANK_SIZE_L;
                        self.chr_bank[5] = self.register[0] * CHR_BANK_SIZE_L + CHR_BANK_SIZE_L;
                        self.chr_bank[6] = (self.register[1] & 0xFE) * CHR_BANK_SIZE_L;
                        self.chr_bank[7] = self.register[1] * CHR_BANK_SIZE_L + CHR_BANK_SIZE_L;
                    } else {
                        self.chr_bank[0] = (self.register[0] & 0xFE) * CHR_BANK_SIZE_L;
                        self.chr_bank[1] = self.register[0] * CHR_BANK_SIZE_L + CHR_BANK_SIZE_L;
                        self.chr_bank[2] = (self.register[1] & 0xFE) * CHR_BANK_SIZE_L;
                        self.chr_bank[3] = self.register[1] * CHR_BANK_SIZE_L + CHR_BANK_SIZE_L;
                        self.chr_bank[4] = self.register[2] * CHR_BANK_SIZE_L;
                        self.chr_bank[5] = self.register[3] * CHR_BANK_SIZE_L;
                        self.chr_bank[6] = self.register[4] * CHR_BANK_SIZE_L;
                        self.chr_bank[7] = self.register[5] * CHR_BANK_SIZE_L;
                    }

                    if self.prg_bank_mode {
                        self.prg_bank[2] = (self.register[6] & 0x3F) * PRG_BANK_SIZE_L;
                        self.prg_bank[0] = ((self.prg_banks as usize) * 2 - 2) * PRG_BANK_SIZE_L;
                    } else {
                        self.prg_bank[0] = (self.register[6] & 0x3F) * PRG_BANK_SIZE_L;
                        self.prg_bank[2] = ((self.prg_banks as usize) * 2 - 2) * PRG_BANK_SIZE_L;
                    }
                    self.prg_bank[1] = (self.register[7] & 0x3F) * PRG_BANK_SIZE_L;
                    self.prg_bank[3] = ((self.prg_banks as usize) * 2 - 1) * PRG_BANK_SIZE_L;
                }
            } else if addr.0 <= 0xBFFF {
                // Mirroring
                if (addr.0 & 0x0001) == 0 {
                    if (data.0 & 0x01) != 0 {
                        self.mirror = MirrorMode::Horizontal;
                    } else {
                        self.mirror = MirrorMode::Vertical;
                    }
                }
            } else if addr.0 <= 0xDFFF {
                // Interrupts
                if (addr.0 & 0x0001) == 0 {
                    self.interrupt_step = data.0 as u16;
                } else {
                    self.interrupt_counter = 0;
                }
            } else {
                // Interrupts
                if (addr.0 & 0x0001) == 0 {
                    self.interrupt_active = false;
                    self.interrupt_enabled = false;
                } else {
                    self.interrupt_enabled = true;
                }
            }
        }
    }

    fn reset(&mut self) {
        self.target_reg = 0;
        self.prg_bank_mode = false;
        self.chr_inversion = false;
        self.mirror = MirrorMode::Horizontal;

        self.interrupt_active = false;
        self.interrupt_enabled = false;
        self.interrupt_counter = 0;
        self.interrupt_step = 0;

        self.register = [0; 8];
        self.chr_bank = [0; 8];
        self.prg_bank = [
            0,
            0x2000,
            ((self.prg_banks as usize) * 2 - 2) * 0x2000,
            ((self.prg_banks as usize) * 2 - 1) * 0x2000,
        ];
    }
}

struct AxRom {
    prg_bank: u8,
    mirror: MirrorMode,
}
impl AxRom {
    fn new() -> Self {
        Self {
            prg_bank: 0,
            mirror: MirrorMode::OneScreenLow,
        }
    }
}
impl Mapper for AxRom {
    fn mirror(&self) -> Option<MirrorMode> {
        Some(self.mirror)
    }

    fn interrupt_state(&self) -> bool {
        false
    }

    fn reset_interrupt(&mut self) {}

    fn on_scanline(&mut self) {}

    fn cpu_read(&self, addr: cpu6502::Address) -> MapperReadResult {
        if addr.0 >= 0x8000 {
            MapperReadResult::Address(Some(
                (self.prg_bank as usize) * 2 * PRG_BANK_SIZE + ((addr.0 & 0x7FFF) as usize),
            ))
        } else {
            MapperReadResult::Address(None)
        }
    }

    fn ppu_read(&self, addr: ppu2C02::Address) -> MapperReadResult {
        if addr.0 <= 0x1FFF {
            MapperReadResult::Address(Some(addr.0 as usize))
        } else {
            MapperReadResult::Address(None)
        }
    }

    fn cpu_write(&mut self, addr: cpu6502::Address, data: cpu6502::Word) {
        if addr.0 >= 0x8000 {
            self.prg_bank = data.0 & 0x07;
            self.mirror = if (data.0 & 0x10) == 0 {
                MirrorMode::OneScreenLow
            } else {
                MirrorMode::OneScreenHigh
            }
        }
    }

    fn reset(&mut self) {
        self.prg_bank = 0;
        self.mirror = MirrorMode::OneScreenLow;
    }
}

struct GxRom {
    prg_bank: u8,
    chr_bank: u8,
}
impl GxRom {
    fn new() -> Self {
        Self {
            prg_bank: 0,
            chr_bank: 0,
        }
    }
}
impl Mapper for GxRom {
    fn mirror(&self) -> Option<MirrorMode> {
        None
    }

    fn interrupt_state(&self) -> bool {
        false
    }

    fn reset_interrupt(&mut self) {}

    fn on_scanline(&mut self) {}

    fn cpu_read(&self, addr: cpu6502::Address) -> MapperReadResult {
        if addr.0 >= 0x8000 {
            MapperReadResult::Address(Some(
                (self.prg_bank as usize) * 2 * PRG_BANK_SIZE + ((addr.0 & 0x7FFF) as usize),
            ))
        } else {
            MapperReadResult::Address(None)
        }
    }

    fn ppu_read(&self, addr: ppu2C02::Address) -> MapperReadResult {
        if addr.0 <= 0x1FFF {
            MapperReadResult::Address(Some(
                (self.chr_bank as usize) * CHR_BANK_SIZE + (addr.0 as usize),
            ))
        } else {
            MapperReadResult::Address(None)
        }
    }

    fn cpu_write(&mut self, addr: cpu6502::Address, data: cpu6502::Word) {
        if addr.0 >= 0x8000 {
            self.chr_bank = data.0 & 0x03;
            self.prg_bank = (data.0 >> 4) & 0x03;
        }
    }

    fn reset(&mut self) {
        self.prg_bank = 0;
        self.chr_bank = 0;
    }
}

fn get_mapper_from_id(id: u8, prg_banks: u8) -> Option<EmuRef<dyn Mapper>> {
    // This is only a very small subset of all existing mappers,
    // but these will enable most Nintendo first-party titles to be emulated
    match id {
        0 => Some(make_ref(NRom::new(prg_banks))),
        1 => Some(make_ref(Mmc1::new(prg_banks))),
        2 => Some(make_ref(UxRom::new(prg_banks))),
        3 => Some(make_ref(CNRom::new(prg_banks))),
        4 => Some(make_ref(Mmc3::new(prg_banks))),
        7 => Some(make_ref(AxRom::new())),
        66 => Some(make_ref(GxRom::new())),
        _ => None,
    }
}

pub struct Cartridge {
    mapper: EmuRef<dyn Mapper>,
    cpu_adapter: EmuRef<CartridgeCpuAdapter>,
    ppu_adapter: EmuRef<CartridgePpuAdapter>,
    mirror: MirrorMode,
}
impl Cartridge {
    const CPU_RANGE: AddressRange<cpu6502::Address> =
        AddressRange::new(Wrapping(0x4020), Wrapping(0xFFFF));
    const PPU_RANGE: AddressRange<cpu6502::Address> =
        AddressRange::new(Wrapping(0x0000), Wrapping(0x1FFF));

    fn new(
        mapper: EmuRef<dyn Mapper>,
        prg_rom: Vec<u8>,
        chr_rom: Vec<u8>,
        chr_is_ram: bool,
        mirror: MirrorMode,
    ) -> Self {
        let cpu_adapter = make_ref(CartridgeCpuAdapter::new(clone_ref(&mapper), prg_rom));
        let ppu_adapter = make_ref(CartridgePpuAdapter::new(
            clone_ref(&mapper),
            chr_rom,
            chr_is_ram,
        ));

        Self {
            mapper,
            cpu_adapter,
            ppu_adapter,
            mirror,
        }
    }

    #[inline]
    fn get_cpu_adapter(&self) -> EmuRef<CartridgeCpuAdapter> {
        clone_ref(&self.cpu_adapter)
    }
    #[inline]
    fn get_ppu_adapter(&self) -> EmuRef<CartridgePpuAdapter> {
        clone_ref(&self.ppu_adapter)
    }

    #[inline]
    fn mirror(&self) -> MirrorMode {
        if let Some(mapper_mirror) = self.mapper.borrow().mirror() {
            mapper_mirror
        } else {
            self.mirror
        }
    }

    #[inline]
    fn reset_mapper(&mut self) {
        self.mapper.borrow_mut().reset();
    }

    #[inline]
    fn interrupt_state(&self) -> bool {
        self.mapper.borrow().interrupt_state()
    }

    #[inline]
    fn reset_interrupt(&mut self) {
        self.mapper.borrow_mut().reset_interrupt();
    }

    #[inline]
    pub fn on_scanline(&mut self) {
        self.mapper.borrow_mut().on_scanline();
    }
}

struct CartridgeCpuAdapter {
    mapper: EmuRef<dyn Mapper>,
    prg_rom: Vec<u8>,
}
impl CartridgeCpuAdapter {
    #[inline]
    const fn new(mapper: EmuRef<dyn Mapper>, prg_rom: Vec<u8>) -> Self {
        Self { mapper, prg_rom }
    }
}
impl BusComponent<cpu6502::Address, cpu6502::Word> for CartridgeCpuAdapter {
    #[inline]
    fn read_range(&self) -> Option<AddressRange<cpu6502::Address>> {
        Some(Cartridge::CPU_RANGE)
    }
    #[inline]
    fn write_range(&self) -> Option<AddressRange<cpu6502::Address>> {
        Some(Cartridge::CPU_RANGE)
    }

    fn read(&mut self, address: cpu6502::Address) -> cpu6502::Word {
        match self
            .mapper
            .borrow()
            .cpu_read(address + Cartridge::CPU_RANGE.start)
        {
            MapperReadResult::Data(data) => data,
            MapperReadResult::Address(Some(mapped_addr)) => Wrapping(self.prg_rom[mapped_addr]),
            _ => Wrapping(0),
        }
    }

    #[inline]
    fn write(&mut self, address: cpu6502::Address, data: cpu6502::Word) {
        self.mapper
            .borrow_mut()
            .cpu_write(address + Cartridge::CPU_RANGE.start, data);
    }
}

struct CartridgePpuAdapter {
    mapper: EmuRef<dyn Mapper>,
    chr_rom: Vec<u8>,
    chr_is_ram: bool,
}
impl CartridgePpuAdapter {
    #[inline]
    const fn new(mapper: EmuRef<dyn Mapper>, chr_rom: Vec<u8>, chr_is_ram: bool) -> Self {
        Self {
            mapper,
            chr_rom,
            chr_is_ram,
        }
    }
}
impl BusComponent<ppu2C02::Address, ppu2C02::Word> for CartridgePpuAdapter {
    #[inline]
    fn read_range(&self) -> Option<AddressRange<ppu2C02::Address>> {
        Some(Cartridge::PPU_RANGE)
    }
    #[inline]
    fn write_range(&self) -> Option<AddressRange<ppu2C02::Address>> {
        Some(Cartridge::PPU_RANGE)
    }

    fn read(&mut self, address: ppu2C02::Address) -> ppu2C02::Word {
        if self.chr_is_ram {
            Wrapping(self.chr_rom[(address.0 & 0x1FFF) as usize])
        } else {
            match self.mapper.borrow().ppu_read(address) {
                MapperReadResult::Data(data) => data,
                MapperReadResult::Address(Some(mapped_addr)) => Wrapping(self.chr_rom[mapped_addr]),
                _ => Wrapping(0),
            }
        }
    }

    #[inline]
    fn write(&mut self, address: ppu2C02::Address, data: ppu2C02::Word) {
        if self.chr_is_ram {
            self.chr_rom[(address.0 & 0x1FFF) as usize] = data.0;
        }
    }
}

struct INesHeader {
    prg_banks: u8,
    chr_banks: u8,
    mapper_1: u8,
    mapper_2: u8,
    _prg_ram_size: u8,
    _tv_system_1: u8,
    _tv_system_2: u8,
}
impl INesHeader {
    pub fn from_reader(reader: &mut BinReader) -> Option<Self> {
        // The file ID is a fixed pattern of 4 bytes that has to match exactly
        let mut file_id: [u8; 4] = [0; 4];
        if reader.read_into(&mut file_id) != 4 {
            return None;
        }
        // This byte pattern resolves to "NES" followed by an MSDOS end-of-file character
        if (file_id[0] != 0x4E)
            || (file_id[1] != 0x45)
            || (file_id[2] != 0x53)
            || (file_id[3] != 0x1A)
        {
            return None;
        }

        let prg_banks = reader.read_byte()?;
        let chr_banks = reader.read_byte()?;
        let mapper_1 = reader.read_byte()?;
        let mapper_2 = reader.read_byte()?;
        let prg_ram_size = reader.read_byte()?;
        let tv_system_1 = reader.read_byte()?;
        let tv_system_2 = reader.read_byte()?;
        let mut unused: [u8; 5] = [0; 5];
        if reader.read_into(&mut unused) != 5 {
            return None;
        }

        Some(Self {
            prg_banks,
            chr_banks,
            mapper_1,
            mapper_2,
            _prg_ram_size: prg_ram_size,
            _tv_system_1: tv_system_1,
            _tv_system_2: tv_system_2,
        })
    }
}

pub fn load_cartridge<P: AsRef<Path>>(file: P) -> Option<EmuRef<Cartridge>> {
    if let Ok(mut reader) = BinReader::from_file(file) {
        if let Some(header) = INesHeader::from_reader(&mut reader) {
            // Skip trainer data if it exists
            if (header.mapper_1 & 0x04) != 0 {
                reader.skip(512);
            }

            let mapper_id = (header.mapper_2 & 0xF0) | (header.mapper_1 >> 4);
            if let Some(mapper) = get_mapper_from_id(mapper_id, header.prg_banks) {
                let mut prg_mem: Vec<u8> = vec![0; header.prg_banks as usize * PRG_BANK_SIZE];
                if reader.read_into(&mut prg_mem) != prg_mem.len() {
                    return None;
                }

                let chr_mem: Vec<u8> = if header.chr_banks == 0 {
                    // We have RAM instead of ROM
                    vec![0; CHR_BANK_SIZE]
                } else {
                    let mut tmp = vec![0; (header.chr_banks as usize) * CHR_BANK_SIZE];
                    if reader.read_into(&mut tmp) != tmp.len() {
                        return None;
                    }
                    tmp
                };

                let mirror = if (header.mapper_1 & 0x01) != 0 {
                    MirrorMode::Vertical
                } else {
                    MirrorMode::Horizontal
                };

                return Some(make_ref(Cartridge::new(
                    mapper,
                    prg_mem,
                    chr_mem,
                    header.chr_banks == 0,
                    mirror,
                )));
            }
        }
    }

    None
}

struct Vram {
    range: AddressRange<ppu2C02::Address>,
    tables: [Ram<ppu2C02::Address, ppu2C02::Word>; 2],
    cartridge: Option<EmuRef<Cartridge>>,
}
impl Vram {
    #[inline]
    fn new(start_address: ppu2C02::Address) -> Self {
        const SIZE: ppu2C02::Address = Wrapping(0x1000);
        const TABLE_SIZE: ppu2C02::Address = Wrapping(0x0400);

        Self {
            range: AddressRange::new(start_address, start_address + SIZE - Wrapping(1)),
            tables: [
                Ram::new(TABLE_SIZE, Wrapping(0)),
                Ram::new(TABLE_SIZE, Wrapping(0)),
            ],
            cartridge: None,
        }
    }

    #[inline]
    fn create(start_address: ppu2C02::Address) -> EmuRef<Self> {
        make_ref(Self::new(start_address))
    }

    #[inline]
    fn set_cartridge(&mut self, cartridge: EmuRef<Cartridge>) {
        self.cartridge = Some(cartridge);
    }

    #[inline]
    fn remove_cartridge(&mut self) {
        self.cartridge = None;
    }
}
impl BusComponent<ppu2C02::Address, ppu2C02::Word> for Vram {
    #[inline]
    fn read_range(&self) -> Option<AddressRange<ppu2C02::Address>> {
        Some(self.range)
    }
    #[inline]
    fn write_range(&self) -> Option<AddressRange<ppu2C02::Address>> {
        Some(self.range)
    }

    fn read(&mut self, address: ppu2C02::Address) -> ppu2C02::Word {
        let table_addr = address & Wrapping(0x03FF);
        if let Some(cartridge) = &self.cartridge {
            match cartridge.borrow().mirror() {
                MirrorMode::Horizontal => {
                    let table_index = (address >> 11).0 & 0x0001;
                    self.tables[table_index as usize].read(table_addr)
                }
                MirrorMode::Vertical => {
                    let table_index = (address >> 10).0 & 0x0001;
                    self.tables[table_index as usize].read(table_addr)
                }
                MirrorMode::OneScreenLow => self.tables[0].read(table_addr),
                MirrorMode::OneScreenHigh => self.tables[1].read(table_addr),
            }
        } else {
            Wrapping(0)
        }
    }

    fn write(&mut self, address: ppu2C02::Address, data: ppu2C02::Word) {
        let table_addr = address & Wrapping(0x03FF);
        if let Some(cartridge) = &self.cartridge {
            match cartridge.borrow().mirror() {
                MirrorMode::Horizontal => {
                    let table_index = (address >> 11).0 & 0x0001;
                    self.tables[table_index as usize].write(table_addr, data);
                }
                MirrorMode::Vertical => {
                    let table_index = (address >> 10).0 & 0x0001;
                    self.tables[table_index as usize].write(table_addr, data);
                }
                MirrorMode::OneScreenLow => self.tables[0].write(table_addr, data),
                MirrorMode::OneScreenHigh => self.tables[1].write(table_addr, data),
            }
        }
    }
}

struct DmaInterface {
    range: AddressRange<cpu6502::Address>,
    page: Wrapping<u8>,
    active: bool,
}
impl DmaInterface {
    #[inline]
    const fn new(address: cpu6502::Address) -> Self {
        Self {
            range: AddressRange::new(address, address),
            page: Wrapping(0),
            active: false,
        }
    }

    #[inline]
    fn create(address: cpu6502::Address) -> EmuRef<Self> {
        make_ref(Self::new(address))
    }
}
impl BusComponent<cpu6502::Address, cpu6502::Word> for DmaInterface {
    #[inline]
    fn read_range(&self) -> Option<AddressRange<cpu6502::Address>> {
        None // Not readable
    }
    #[inline]
    fn write_range(&self) -> Option<AddressRange<cpu6502::Address>> {
        Some(self.range)
    }

    #[inline]
    fn read(&mut self, _address: cpu6502::Address) -> cpu6502::Word {
        Wrapping(0) // Not readable
    }

    #[inline]
    fn write(&mut self, _address: cpu6502::Address, data: cpu6502::Word) {
        self.page = data;
        self.active = true;
    }
}

bitflags! {
    pub struct Buttons : u8 {
        const A      = 0b10000000;
        const B      = 0b01000000;
        const SELECT = 0b00100000;
        const START  = 0b00010000;
        const UP     = 0b00001000;
        const DOWN   = 0b00000100;
        const LEFT   = 0b00000010;
        const RIGHT  = 0b00000001;
    }
}

struct VController {
    range: AddressRange<cpu6502::Address>,
    controller: [u8; 2],
    buffer: [Buttons; 2],
}
impl VController {
    #[inline]
    fn new(start_address: cpu6502::Address) -> Self {
        Self {
            range: AddressRange::new(start_address, start_address + Wrapping(1)),
            controller: [0; 2],
            buffer: [Buttons::empty(); 2],
        }
    }

    #[inline]
    fn create(start_address: cpu6502::Address) -> EmuRef<Self> {
        make_ref(Self::new(start_address))
    }

    #[inline]
    fn update_state(&mut self, controller_0: Buttons, controller_1: Buttons) {
        self.buffer[0] = controller_0;
        self.buffer[1] = controller_1;
    }
}
impl BusComponent<cpu6502::Address, cpu6502::Word> for VController {
    #[inline]
    fn read_range(&self) -> Option<AddressRange<cpu6502::Address>> {
        Some(self.range)
    }
    #[inline]
    fn write_range(&self) -> Option<AddressRange<cpu6502::Address>> {
        Some(self.range)
    }

    #[inline]
    fn read(&mut self, address: cpu6502::Address) -> cpu6502::Word {
        // Reading is sequential
        let result = self.controller[address.0 as usize] >> 7;
        self.controller[address.0 as usize] <<= 1;
        Wrapping(result)
    }

    #[inline]
    fn write(&mut self, address: cpu6502::Address, _data: cpu6502::Word) {
        // Cannot write to the controllers, instead this stores the buffer
        self.controller[address.0 as usize] = self.buffer[address.0 as usize].bits();
    }
}
