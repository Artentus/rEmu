use crate::bus::*;
use crate::types::HardwareInteger;
use crate::*;
use std::marker::PhantomData;

pub struct Ram<TAddress, TWord>
where
    TAddress: HardwareInteger,
    TWord: HardwareInteger,
{
    data: Vec<TWord>,
    range: AddressRange<TAddress>,
    phantom: PhantomData<TAddress>,
}
impl<TAddress, TWord> Ram<TAddress, TWord>
where
    TAddress: HardwareInteger,
    TWord: HardwareInteger,
{
    pub fn new(size: TAddress, start_address: TAddress) -> Self {
        Self {
            data: vec![TWord::zero(); size.to_usize().unwrap()],
            range: AddressRange::new(start_address, start_address + size - TAddress::one()),
            phantom: PhantomData,
        }
    }

    #[inline]
    pub fn create(size: TAddress, start_address: TAddress) -> EmuRef<Self> {
        make_ref(Self::new(size, start_address))
    }
}
impl<TAddress, TWord> BusComponent<TAddress, TWord> for Ram<TAddress, TWord>
where
    TAddress: HardwareInteger,
    TWord: HardwareInteger,
{
    #[inline]
    fn read_range(&self) -> Option<AddressRange<TAddress>> {
        Some(self.range)
    }
    #[inline]
    fn write_range(&self) -> Option<AddressRange<TAddress>> {
        Some(self.range)
    }

    #[inline]
    fn read(&mut self, address: TAddress) -> TWord {
        self.data[address.to_usize().unwrap()]
    }

    #[inline]
    fn write(&mut self, address: TAddress, data: TWord) {
        self.data[address.to_usize().unwrap()] = data;
    }
}
