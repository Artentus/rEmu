use crate::types::HardwareInteger;
use crate::*;
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AddressRange<TAddress>
where
    TAddress: HardwareInteger,
{
    /// First address in the range
    pub start: TAddress,
    /// Last address in the range
    pub end: TAddress,
}
impl<TAddress> AddressRange<TAddress>
where
    TAddress: HardwareInteger,
{
    #[inline]
    pub const fn new(start: TAddress, end: TAddress) -> Self {
        Self { start, end }
    }

    #[inline]
    /// The length of the range
    pub fn len(&self) -> TAddress {
        self.end - self.start + TAddress::one()
    }

    /// Checks whether a given address falls within the range
    #[inline]
    pub fn contains(&self, address: TAddress) -> bool {
        (address >= self.start) && (address <= self.end)
    }
}

/// A hardware component that is connected to a bus
pub trait BusComponent<TAddress, TWord>
where
    TAddress: HardwareInteger,
    TWord: HardwareInteger,
{
    /// The CPU address range at which this component is active when reading
    fn read_range(&self) -> Option<AddressRange<TAddress>>;
    /// The CPU address range at which this component is active when writing
    fn write_range(&self) -> Option<AddressRange<TAddress>>;

    /// Reads from the component
    ///
    /// The address is given relative to the components address space (CPU address - read range start)
    fn read(&mut self, address: TAddress) -> TWord;
    /// Writes to the component
    ///
    /// The address is given relative to the components address space (CPU address - write range start)
    fn write(&mut self, address: TAddress, data: TWord);
}

pub type BusRef<'a, TAddress, TWord> = EmuRef<dyn BusComponent<TAddress, TWord> + 'a>;

/// Expands the address range of a bus component by mirroring
pub struct MirroredBusComponent<'a, TAddress, TWord>
where
    TAddress: HardwareInteger,
    TWord: HardwareInteger,
{
    base_component: BusRef<'a, TAddress, TWord>,
    read_range: Option<AddressRange<TAddress>>,
    write_range: Option<AddressRange<TAddress>>,
    read_mod: TAddress,
    write_mod: TAddress,
}
impl<'a, TAddress, TWord> MirroredBusComponent<'a, TAddress, TWord>
where
    TAddress: HardwareInteger,
    TWord: HardwareInteger,
{
    pub fn new(
        base_component: BusRef<'a, TAddress, TWord>,
        new_read_end: TAddress,
        new_write_end: TAddress,
    ) -> Self {
        let (base_read_range, base_write_range) = {
            let base_component_borrow = base_component.borrow();
            (
                base_component_borrow.read_range(),
                base_component_borrow.write_range(),
            )
        };

        Self {
            base_component,
            read_range: base_read_range.map(|r| AddressRange::new(r.start, new_read_end)),
            write_range: base_write_range.map(|r| AddressRange::new(r.start, new_write_end)),
            read_mod: base_read_range.map_or(TAddress::one(), |r| r.len()),
            write_mod: base_write_range.map_or(TAddress::one(), |r| r.len()),
        }
    }

    #[inline]
    pub fn create(
        base_component: BusRef<'a, TAddress, TWord>,
        new_read_end: TAddress,
        new_write_end: TAddress,
    ) -> EmuRef<Self> {
        make_ref(Self::new(base_component, new_read_end, new_write_end))
    }
}
impl<'a, TAddress, TWord> BusComponent<TAddress, TWord>
    for MirroredBusComponent<'a, TAddress, TWord>
where
    TAddress: HardwareInteger,
    TWord: HardwareInteger,
{
    #[inline]
    fn read_range(&self) -> Option<AddressRange<TAddress>> {
        self.read_range
    }
    #[inline]
    fn write_range(&self) -> Option<AddressRange<TAddress>> {
        self.write_range
    }

    #[inline]
    fn read(&mut self, address: TAddress) -> TWord {
        self.base_component
            .borrow_mut()
            .read(address % self.read_mod)
    }
    #[inline]
    fn write(&mut self, address: TAddress, data: TWord) {
        self.base_component
            .borrow_mut()
            .write(address % self.write_mod, data)
    }
}

pub fn mirror_component<'a, TAddress: 'a, TWord: 'a>(
    component: BusRef<'a, TAddress, TWord>,
    end_address: TAddress,
) -> BusRef<'a, TAddress, TWord>
where
    TAddress: HardwareInteger,
    TWord: HardwareInteger,
{
    MirroredBusComponent::create(component, end_address, end_address)
}

pub type BusHandle = u32;

pub struct Bus<'a, TAddress, TWord>
where
    TAddress: HardwareInteger,
    TWord: HardwareInteger,
{
    components: HashMap<BusHandle, BusRef<'a, TAddress, TWord>>,
    next_handle: BusHandle,
}
impl<'a, TAddress, TWord> Bus<'a, TAddress, TWord>
where
    TAddress: HardwareInteger,
    TWord: HardwareInteger,
{
    #[inline]
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
            next_handle: 0,
        }
    }

    #[inline]
    pub fn create() -> EmuRef<Self> {
        make_ref(Self::new())
    }

    #[inline]
    pub fn add_component(&mut self, component: BusRef<'a, TAddress, TWord>) -> BusHandle {
        let handle = self.next_handle;
        self.components.insert(handle, component);
        self.next_handle += 1;
        handle
    }

    #[inline]
    pub fn remove_component(&mut self, handle: BusHandle) -> Option<BusRef<'a, TAddress, TWord>> {
        self.components.remove(&handle)
    }

    pub fn read(&self, address: TAddress) -> TWord {
        let mut result = TWord::zero();

        for (_, component_ref) in self.components.iter() {
            if let Ok(mut component) = component_ref.try_borrow_mut() {
                if let Some(range) = component.read_range() {
                    if range.contains(address) {
                        result |= component.read(address - range.start);
                    }
                }
            }
        }

        result
    }

    pub fn write(&self, address: TAddress, data: TWord) {
        for (_, component_ref) in self.components.iter() {
            if let Ok(mut component) = component_ref.try_borrow_mut() {
                if let Some(range) = component.write_range() {
                    if range.contains(address) {
                        component.write(address - range.start, data);
                    }
                }
            }
        }
    }
}
