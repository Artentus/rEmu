pub mod cpu6502;
#[allow(non_snake_case)]
pub mod cpu65C816;

use crate::types::HardwareInteger;
use crate::*;

pub trait AsmInstruction<TAddress>: Display
where
    TAddress: HardwareInteger,
{
    fn address(&self) -> TAddress;
    fn byte_size(&self) -> usize;
    fn mnemonic(&self) -> &str;
}

pub trait Cpu<TAddress, TWord, TInstruction>: Display
where
    TAddress: HardwareInteger,
    TWord: HardwareInteger,
    TInstruction: AsmInstruction<TAddress>,
{
    fn reset(&mut self) -> u32;

    fn execute_next_instruction(&mut self) -> u32;

    fn disassemble_current(&self, range: usize) -> Box<[TInstruction]>;

    fn execute_cycles(&mut self, cycles: u32) -> u32 {
        let mut run: u32 = 0;
        while run < cycles {
            run += self.execute_next_instruction();
        }
        run
    }
}
