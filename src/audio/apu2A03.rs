use crate::audio::*;
use crate::bus::AddressRange;
use crate::cpu::cpu6502;
use crate::*;

trait Channel {
    fn write(&mut self, address: u8, data: u8);
    fn clock(&mut self, quarter: bool, half: bool);
    fn sample(&mut self) -> f32;
}

struct Sequencer {
    period: u16,
    timer: Wrapping<u16>,
}
impl Sequencer {
    #[inline]
    const fn new() -> Self {
        Self {
            period: 0,
            timer: Wrapping(0),
        }
    }

    #[inline]
    const fn is_pulse_enabled(&self) -> bool {
        (self.period & 0x07FF) >= 8
    }

    #[inline]
    const fn is_triangle_enabled(&self) -> bool {
        (self.period & 0x07FF) >= 2
    }

    #[inline]
    fn set_lo(&mut self, lo: u8) {
        self.period = (self.period & 0xFF00) | (lo as u16);
    }

    #[inline]
    fn set_hi(&mut self, hi: u8) {
        self.period = (((hi & 0x07) as u16) << 8) | (self.period & 0xF8FF);
        self.timer = Wrapping(self.period & 0x07FF);
    }

    #[inline]
    fn set_period(&mut self, period: u16) {
        self.period = (self.period & 0xF800) | (period & 0x07FF);
        self.timer = Wrapping(self.period & 0x07FF);
    }

    fn clock(&mut self) -> bool {
        self.timer -= Wrapping(1);

        if self.timer.0 == 0xFFFF {
            self.timer = Wrapping(self.period & 0x07FF);
            true
        } else {
            false
        }
    }
}

struct LengthCounter {
    halt: bool,
    counter: u8,
}
impl LengthCounter {
    #[inline]
    const fn new() -> Self {
        Self {
            halt: false,
            counter: 0,
        }
    }

    #[inline]
    fn load(&mut self, value: u8) {
        const LOAD_TABLE: [u8; 0x20] = [
            10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, 12, 16, 24, 18, 48, 20,
            96, 22, 192, 24, 72, 26, 16, 28, 32, 30,
        ];

        self.counter = LOAD_TABLE[((value & 0xF8) >> 3) as usize];
    }

    #[inline]
    fn clock(&mut self) {
        if (self.counter > 0) && !self.halt {
            self.counter -= 1;
        }
    }
}

const VOLUME_SCALE: f32 = 15.0;

struct Envelope {
    length_counter: LengthCounter,

    use_constant_volume: bool,
    volume_or_reload: u8,
    start: bool,
    divider_counter: u8,
    decay_counter: u8,
}
impl Envelope {
    #[inline]
    const fn new() -> Self {
        Self {
            length_counter: LengthCounter::new(),
            use_constant_volume: true,
            volume_or_reload: 0,
            start: false,
            divider_counter: 0,
            decay_counter: 0,
        }
    }

    fn get_volume(&self) -> f32 {
        if self.length_counter.counter > 0 {
            if self.use_constant_volume {
                (self.volume_or_reload as f32) / VOLUME_SCALE
            } else {
                (self.decay_counter as f32) / VOLUME_SCALE
            }
        } else {
            0.0
        }
    }

    #[inline]
    fn set(&mut self, value: u8) {
        self.use_constant_volume = (value & 0x10) != 0;
        self.volume_or_reload = value & 0x0F;
        self.start = true;
    }

    fn clock(&mut self) {
        if self.start {
            self.start = false;
            self.decay_counter = 15;
            self.divider_counter = self.volume_or_reload;
        } else {
            if self.divider_counter == 0 {
                self.divider_counter = self.volume_or_reload;

                if self.decay_counter == 0 {
                    if self.length_counter.halt {
                        self.decay_counter = 15;
                    }
                } else {
                    self.decay_counter -= 1;
                }
            } else {
                self.divider_counter -= 1;
            }
        }
    }
}

struct Sweep {
    is_channel_1: bool,
    enabled: bool,
    period: u8,
    negate: bool,
    shift: u8,
    reload: bool,
    divider: u8,
    target_period: u32,
    real_period: u16,
}
impl Sweep {
    #[inline]
    const fn new(is_channel_1: bool) -> Self {
        Self {
            is_channel_1,
            enabled: false,
            period: 0,
            negate: false,
            shift: 0,
            reload: false,
            divider: 0,
            target_period: 0,
            real_period: 0,
        }
    }

    fn update_target_period(&mut self) {
        let shift_result = self.real_period >> self.shift;
        if self.negate {
            self.target_period = (self.real_period - shift_result) as u32;
            if self.is_channel_1 {
                self.target_period -= 1;
            }
        } else {
            self.target_period = (self.real_period + shift_result) as u32;
        }
    }

    fn set(&mut self, value: u8) {
        self.enabled = (value & 0x80) != 0;
        self.period = (value & 0x70) >> 4;
        self.negate = (value & 0x08) != 0;
        self.shift = value & 0x07;
        self.reload = true;

        self.update_target_period();
    }

    fn clock(&mut self, target: &mut u16) {
        self.divider -= 1;
        if self.divider == 0 {
            if (self.shift > 0)
                && self.enabled
                && (self.real_period >= 8)
                && (self.target_period <= 0x07FF)
            {
                self.real_period = (self.target_period & 0xFFFF) as u16;
                *target = self.real_period * 2 + 1;
            }
            self.divider = self.period;
        }

        if self.reload {
            self.divider = self.period;
            self.reload = false;
        }
    }
}

struct PulseChannel {
    sequence: u8,
    sequence_pos: u8,
    enabled: bool,
    sequencer: Sequencer,
    envelope: Envelope,
    sweep: Sweep,
}
impl PulseChannel {
    const SEQUENCES: [u8; 4] = [0b00000001, 0b00000011, 0b00001111, 0b11111100];

    const fn new(is_channel_1: bool) -> Self {
        Self {
            sequence: Self::SEQUENCES[0],
            sequence_pos: 0,
            enabled: true,
            sequencer: Sequencer::new(),
            envelope: Envelope::new(),
            sweep: Sweep::new(is_channel_1),
        }
    }
}
impl Channel for PulseChannel {
    fn write(&mut self, address: u8, data: u8) {
        match address {
            0 => {
                let sequence_index = ((data & 0xC0) >> 6) as usize;
                self.sequence = Self::SEQUENCES[sequence_index];
                self.envelope.length_counter.halt = (data & 0x20) != 0;
                self.envelope.set(data);
            }
            1 => {
                self.sweep.set(data);
            }
            2 => {
                self.sequencer.set_lo(data);
            }
            3 => {
                self.sequencer.set_hi(data);
                self.envelope.length_counter.load(data);
                self.envelope.start = true;
            }
            _ => {
                panic!("Invalid channel register")
            }
        }
    }

    fn clock(&mut self, quarter: bool, half: bool) {
        if quarter {
            self.envelope.clock();
        }

        if half {
            self.envelope.length_counter.clock();
            self.sweep.clock(&mut self.sequencer.period);
        }

        if self.sequencer.clock() {
            self.sequence_pos = (self.sequence_pos + 1) & 0x07;
        }
    }

    fn sample(&mut self) -> f32 {
        if self.enabled && self.sequencer.is_pulse_enabled() {
            let mask: u8 = 0x01 << self.sequence_pos;
            let output = (self.sequence & mask) >> self.sequence_pos;
            (output as f32) * self.envelope.get_volume()
        } else {
            0.5
        }
    }
}

struct TriangleChannel {
    sequence_pos: u8,
    enabled: bool,
    sequencer: Sequencer,
    length_counter: LengthCounter,
    linear_counter: u8,
    linear_counter_reload: u8,
    reload: bool,
}
impl TriangleChannel {
    const fn new() -> Self {
        Self {
            sequence_pos: 0,
            enabled: true,
            sequencer: Sequencer::new(),
            length_counter: LengthCounter::new(),
            linear_counter: 0,
            linear_counter_reload: 0,
            reload: false,
        }
    }
}
impl Channel for TriangleChannel {
    fn write(&mut self, address: u8, data: u8) {
        match address {
            0 => {
                self.length_counter.halt = (data & 0x80) != 0;
                self.linear_counter_reload = data & 0x7F;
            }
            1 => {}
            2 => {
                self.sequencer.set_lo(data);
            }
            3 => {
                self.sequencer.set_hi(data);
                self.length_counter.load(data);
                self.reload = true;
            }
            _ => {
                panic!("Invalid channel register")
            }
        }
    }

    fn clock(&mut self, quarter: bool, half: bool) {
        if quarter {
            if self.reload {
                self.linear_counter = self.linear_counter_reload;
            } else if self.linear_counter > 0 {
                self.linear_counter -= 1;
            }

            if !self.length_counter.halt {
                self.reload = false;
            }
        }

        if half {
            self.length_counter.clock();
        }

        if self.sequencer.clock() {
            self.sequence_pos = (self.sequence_pos + 1) & 0x1F;
        }
    }

    fn sample(&mut self) -> f32 {
        const SEQUENCE: [f32; 32] = [
            15.0 / VOLUME_SCALE,
            14.0 / VOLUME_SCALE,
            13.0 / VOLUME_SCALE,
            12.0 / VOLUME_SCALE,
            11.0 / VOLUME_SCALE,
            10.0 / VOLUME_SCALE,
            9.0 / VOLUME_SCALE,
            8.0 / VOLUME_SCALE,
            7.0 / VOLUME_SCALE,
            6.0 / VOLUME_SCALE,
            5.0 / VOLUME_SCALE,
            4.0 / VOLUME_SCALE,
            3.0 / VOLUME_SCALE,
            2.0 / VOLUME_SCALE,
            1.0 / VOLUME_SCALE,
            0.0 / VOLUME_SCALE,
            0.0 / VOLUME_SCALE,
            1.0 / VOLUME_SCALE,
            2.0 / VOLUME_SCALE,
            3.0 / VOLUME_SCALE,
            4.0 / VOLUME_SCALE,
            5.0 / VOLUME_SCALE,
            6.0 / VOLUME_SCALE,
            7.0 / VOLUME_SCALE,
            8.0 / VOLUME_SCALE,
            9.0 / VOLUME_SCALE,
            10.0 / VOLUME_SCALE,
            11.0 / VOLUME_SCALE,
            12.0 / VOLUME_SCALE,
            13.0 / VOLUME_SCALE,
            14.0 / VOLUME_SCALE,
            15.0 / VOLUME_SCALE,
        ];

        if self.enabled
            && self.sequencer.is_triangle_enabled()
            && (self.length_counter.counter > 0)
            && (self.linear_counter > 0)
        {
            SEQUENCE[self.sequence_pos as usize]
        } else {
            0.5
        }
    }
}

struct NoiseChannel {
    enabled: bool,
    shift: u16,
    mode: bool,
    feedback: u16,
    sequencer: Sequencer,
    envelope: Envelope,
}
impl NoiseChannel {
    const fn new() -> Self {
        Self {
            enabled: true,
            shift: 0x0001,
            mode: false,
            feedback: 0,
            sequencer: Sequencer::new(),
            envelope: Envelope::new(),
        }
    }
}
impl Channel for NoiseChannel {
    fn write(&mut self, address: u8, data: u8) {
        const PERIOD_LOOKUP: [u16; 16] = [
            4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
        ];

        match address {
            0 => {
                self.envelope.length_counter.halt = (data & 0x20) != 0;
                self.envelope.set(data);
            }
            1 => {}
            2 => {
                self.mode = (data & 0x80) != 0;
                self.sequencer
                    .set_period(PERIOD_LOOKUP[(data & 0x0F) as usize] - 1);
            }
            3 => {
                self.envelope.length_counter.load(data);
                self.envelope.start = true;
            }
            _ => {
                panic!("Invalid channel register")
            }
        }
    }

    fn clock(&mut self, quarter: bool, half: bool) {
        if quarter {
            self.envelope.clock();
        }

        if half {
            self.envelope.length_counter.clock();
        }

        if self.sequencer.clock() {
            let bit_1 = self.shift & 0x0001;
            let bit_2 = if self.mode {
                (self.shift & 0x0040) >> 6
            } else {
                (self.shift & 0x0002) >> 1
            };
            self.feedback = bit_1 ^ bit_2;
            self.shift |= self.feedback << 15;
            self.shift = (self.shift & 0xFFFE) >> 1;
        }
    }

    fn sample(&mut self) -> f32 {
        if self.enabled && ((self.shift & 0x0001) != 0) {
            (self.feedback as f32) * self.envelope.get_volume()
        } else {
            0.5
        }
    }
}

struct DmcChannel {}
impl DmcChannel {
    const fn new() -> Self {
        Self {}
    }
}
impl Channel for DmcChannel {
    fn write(&mut self, address: u8, data: u8) {}

    fn clock(&mut self, quarter: bool, half: bool) {}

    fn sample(&mut self) -> f32 {
        0.0
    }
}

pub struct Apu2A03 {
    range: AddressRange<cpu6502::Address>,
    pulse_channel_1: PulseChannel,
    pulse_channel_2: PulseChannel,
    triangle_channel: TriangleChannel,
    noise_channel: NoiseChannel,
    dmc_channel: DmcChannel,
    even_cycle: bool,
    cycles: u32,
    t: f32,
}
impl Apu2A03 {
    const SECONDS_PER_CLOCK: f32 = 1.0 / (NES_APU_CLOCK as f32);

    pub fn new(range_start: cpu6502::Address) -> Self {
        const MAX_ADDRESS: cpu6502::Address = Wrapping(0x0013);

        let pulse_channel_1 = PulseChannel::new(true);
        let pulse_channel_2 = PulseChannel::new(false);
        let triangle_channel = TriangleChannel::new();
        let noise_channel = NoiseChannel::new();
        let dmc_channel = DmcChannel::new();

        Self {
            range: AddressRange::new(range_start, range_start + MAX_ADDRESS),
            pulse_channel_1,
            pulse_channel_2,
            triangle_channel,
            noise_channel,
            dmc_channel,
            even_cycle: false,
            cycles: 0,
            t: 0.0,
        }
    }

    #[inline]
    pub fn create(range_start: cpu6502::Address) -> EmuRef<Self> {
        make_ref(Self::new(range_start))
    }

    fn clock_one(&mut self, buffer: &mut SampleBuffer) {
        self.even_cycle = !self.even_cycle;

        if self.even_cycle {
            self.cycles += 1;
        }

        let full = self.cycles == 14916;
        let half = (self.cycles == 7457) || full;
        let quarter = (self.cycles == 3729) || (self.cycles == 11186) || half;
        if full {
            self.cycles = 0
        }

        self.triangle_channel
            .clock(quarter & self.even_cycle, half & self.even_cycle);

        if self.even_cycle {
            self.pulse_channel_1.clock(quarter, half);
            self.pulse_channel_2.clock(quarter, half);
            self.noise_channel.clock(quarter, half);
            self.dmc_channel.clock(quarter, half);

            self.t += Self::SECONDS_PER_CLOCK;
            while self.t >= 0.0 {
                self.t -= SECONDS_PER_SAMPLE;

                let pulse_1_sample = self.pulse_channel_1.sample();
                let pulse_2_sample = self.pulse_channel_2.sample();
                let triangle_sample = self.triangle_channel.sample();
                let noise_sample = 0.5; // todo: self.noise_channel.sample();
                let dmc_sample = 0.5; // todo: self.dmc_channel.sample();

                let sample = (0.00752 * (pulse_1_sample + pulse_2_sample))
                    + (0.00851 * triangle_sample)
                    + (0.00494 * noise_sample)
                    + (0.00335 * dmc_sample);
                buffer.write((sample * VOLUME_SCALE) * 2.0 - 1.0); // convert from [0,1] to [-1,1]
            }
        }
    }
}
impl BusComponent<cpu6502::Address, cpu6502::Word> for Apu2A03 {
    #[inline]
    fn read_range(&self) -> Option<bus::AddressRange<cpu6502::Address>> {
        None
    }
    #[inline]
    fn write_range(&self) -> Option<bus::AddressRange<cpu6502::Address>> {
        Some(self.range)
    }

    #[inline]
    fn read(&mut self, _address: cpu6502::Address) -> cpu6502::Word {
        Wrapping(0)
    }

    #[inline]
    fn write(&mut self, address: cpu6502::Address, data: cpu6502::Word) {
        let channel_index = (address.0 / 4) as usize;
        let channel_address = (address.0 % 4) as u8;
        match channel_index {
            0 => self.pulse_channel_1.write(channel_address, data.0),
            1 => self.pulse_channel_2.write(channel_address, data.0),
            2 => self.triangle_channel.write(channel_address, data.0),
            3 => self.noise_channel.write(channel_address, data.0),
            4 => self.dmc_channel.write(channel_address, data.0),
            _ => {}
        }
    }
}
impl<'a> AudioChip<'a, cpu6502::Address, cpu6502::Word> for Apu2A03 {
    fn reset(&mut self) {
        self.pulse_channel_1.enabled = false;
        self.pulse_channel_1.envelope.length_counter.counter = 0;

        self.pulse_channel_2.enabled = false;
        self.pulse_channel_2.envelope.length_counter.counter = 0;

        self.triangle_channel.enabled = false;
        self.triangle_channel.length_counter.counter = 0;

        self.noise_channel.enabled = false;
        self.noise_channel.envelope.length_counter.counter = 0;
    }

    fn clock(&mut self, cycles: u32, buffer: &mut SampleBuffer) {
        for _ in 0..cycles {
            self.clock_one(buffer);
        }
    }
}

pub struct Apu2A03Control {
    range: AddressRange<cpu6502::Address>,
    apu: EmuRef<Apu2A03>,
}
impl Apu2A03Control {
    #[inline]
    pub const fn new(range_start: cpu6502::Address, apu: EmuRef<Apu2A03>) -> Self {
        Self {
            range: AddressRange::new(range_start, range_start),
            apu,
        }
    }

    #[inline]
    pub fn create(range_start: cpu6502::Address, apu: EmuRef<Apu2A03>) -> EmuRef<Self> {
        make_ref(Self::new(range_start, apu))
    }
}
impl BusComponent<cpu6502::Address, cpu6502::Word> for Apu2A03Control {
    #[inline]
    fn read_range(&self) -> Option<AddressRange<cpu6502::Address>> {
        Some(self.range)
    }
    #[inline]
    fn write_range(&self) -> Option<AddressRange<cpu6502::Address>> {
        Some(self.range)
    }

    fn read(&mut self, _address: cpu6502::Address) -> cpu6502::Word {
        let mut result: u8 = 0x00;

        let apu_borrow = self.apu.borrow();

        if apu_borrow.pulse_channel_1.envelope.length_counter.counter > 0 {
            result |= 0x01
        }
        if apu_borrow.pulse_channel_2.envelope.length_counter.counter > 0 {
            result |= 0x02
        }
        if apu_borrow.triangle_channel.length_counter.counter > 0 {
            result |= 0x04
        }
        if apu_borrow.noise_channel.envelope.length_counter.counter > 0 {
            result |= 0x08
        }

        Wrapping(result)
    }

    fn write(&mut self, _address: cpu6502::Address, data: cpu6502::Word) {
        let pulse_1_enabled = (data.0 & 0x01) != 0;
        let pulse_2_enabled = (data.0 & 0x02) != 0;
        let triangle_enabled = (data.0 & 0x04) != 0;
        let noise_enabled = (data.0 & 0x08) != 0;
        // todo: let dmc_enabled = (data.0 & 0x10) != 0;

        let mut apu_borrow = self.apu.borrow_mut();

        apu_borrow.pulse_channel_1.enabled = pulse_1_enabled;
        if !pulse_1_enabled {
            apu_borrow.pulse_channel_1.envelope.length_counter.counter = 0
        }

        apu_borrow.pulse_channel_2.enabled = pulse_2_enabled;
        if !pulse_2_enabled {
            apu_borrow.pulse_channel_2.envelope.length_counter.counter = 0
        }

        apu_borrow.triangle_channel.enabled = triangle_enabled;
        if !triangle_enabled {
            apu_borrow.triangle_channel.length_counter.counter = 0
        }

        apu_borrow.noise_channel.enabled = noise_enabled;
        if !noise_enabled {
            apu_borrow.noise_channel.envelope.length_counter.counter = 0
        }
    }
}
