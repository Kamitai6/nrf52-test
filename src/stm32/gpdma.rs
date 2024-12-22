use stm32h5::stm32h562::{GPDMA1, GPDMA2};
use cortex_m::interrupt;

macro_rules! generate_channel_accessors {
    ($instance:ident, $channel_number:expr) => {
        match $channel_number {
            0 => &$instance.ch1cr,
            1 => &$instance.ch2cr,
            2 => &$instance.ch3cr,
            3 => &$instance.ch4cr,
            4 => &$instance.ch5cr,
            5 => &$instance.ch6cr,
            6 => &$instance.ch7cr,
            7 => &$instance.ch8cr,
            _ => unreachable!(),
        }
    };
}

trait Instance {}
impl Instance for GPDMA1 {}
impl Instance for GPDMA2 {}

pub struct Gpdma<'a, T: Instance> {
    instance: &'a T,
    channel: u8,
}

impl<'a> Gpdma<'a, T> {
    pub fn new(instance: &'a T, channel: u8) -> Option<Self> {
        if channel < 8 {
            Some(Self {
                instance,
                channel,
            })
        } else {
            None
        }
    }

    pub fn configure(&self, config: ChannelConfig) {
        let reg = generate_channel_accessors!(self.instance, self.channel_number);
        reg.modify(|_, w| {
            // 必要な設定を記述
            w
        });
    }
}

！！！！！！上と下を統合したい！！！！！！

static mut TAKE_FLAG: [bool; 8] = [false; 8];

// Optional is f**k, so i only use Result
impl<'a, Instance> Gpdma<'a, Instance> {
    #[inline]
    pub fn take(channel: u8, peripheral: &stm32h562::Peripherals) -> Option<Self> {
        interrupt::free(|_| { //割り込み禁止
            if unsafe { TAKE_FLAG[channel] } {
                return None;
            }
            Some(unsafe { Gpdma::steal(channel, peripheral) })
        })
    }
    pub unsafe fn steal(channel: u8, dp: &stm32h562::Peripherals) -> Self {
        TAKE_FLAG[channel] = true;
        crate::interrupt::typelevel::$irq::set_priority_with_cs(cs, irq_priority);
        crate::interrupt::typelevel::$irq::enable();

        crate::pac::RCC.ahb1enr().modify(|w| w.set_gpdma1en(true));
        crate::pac::RCC.ahb1enr().modify(|w| w.set_gpdma2en(true));

        self.dp.GPDMA1.ch3.cr().write(|w| {
            w.dir().from_memory();
            w.psize().bits16();
            w.msize().bits16();
            w.pl().low();
            w.mem2mem().disabled();
            w.pinc().enabled();
            w.minc().disabled();
            w.circ().disabled();
            w.pburst().single();
            w.mburst().single();
            w.chsel().bits(0b011);
        });

        Gpdma { dp }
    }

    pub fn spi3_begin(&self) {
        self.dp.GPIOA.odr().modify(|_, w| w.od15().low());
    }

    pub fn spi3_end(&self) {
        self.dp.GPIOA.odr().modify(|_, w| w.od15().high());
    }

    pub fn spi3_send(&self, data: u16) -> u16 {
        // check_errors!(self.dp.SPI3.sr().read());

        self.dp.SPI3.cr1().modify(|_, w| w.cstart().started());

        while self.dp.SPI3.sr().read().txp().is_full() {
            cortex_m::asm::nop();
        }
        self.dp.SPI3.txdr().write(|w| w.txdr().bits(data as u32));
        while self.dp.SPI3.sr().read().txc().is_ongoing() {
            cortex_m::asm::nop();
        }
        while self.dp.SPI3.sr().read().rxp().is_empty() {
            cortex_m::asm::nop();
        }
        let res = self.dp.SPI3.rxdr().read().bits();
        let resres = (res >> 16) as u16;

        self.dp.SPI3.ifcr().write(|w| w.eotc().set_bit());
        self.dp.SPI3.ifcr().write(|w| w.txtfc().set_bit());
        self.dp.SPI3.ier().reset();

        resres
    }
}

/// DMA transfer.
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Transfer<'a> {
    channel: PeripheralRef<'a, AnyChannel>,
}

impl<'a> Transfer<'a> {
    /// Create a new read DMA transfer (peripheral to memory).
    pub unsafe fn new_read<W: Word>(
        channel: impl Peripheral<P = impl Channel> + 'a,
        request: Request,
        peri_addr: *mut W,
        buf: &'a mut [W],
        options: TransferOptions,
    ) -> Self {
        Self::new_read_raw(channel, request, peri_addr, buf, options)
    }

    /// Create a new read DMA transfer (peripheral to memory), using raw pointers.
    pub unsafe fn new_read_raw<W: Word>(
        channel: impl Peripheral<P = impl Channel> + 'a,
        request: Request,
        peri_addr: *mut W,
        buf: *mut [W],
        options: TransferOptions,
    ) -> Self {
        into_ref!(channel);

        Self::new_inner(
            channel.map_into(),
            request,
            Dir::PeripheralToMemory,
            peri_addr as *const u32,
            buf as *mut W as *mut u32,
            buf.len(),
            true,
            W::size(),
            options,
        )
    }

    /// Create a new write DMA transfer (memory to peripheral).
    pub unsafe fn new_write<W: Word>(
        channel: impl Peripheral<P = impl Channel> + 'a,
        request: Request,
        buf: &'a [W],
        peri_addr: *mut W,
        options: TransferOptions,
    ) -> Self {
        Self::new_write_raw(channel, request, buf, peri_addr, options)
    }

    /// Create a new write DMA transfer (memory to peripheral), using raw pointers.
    pub unsafe fn new_write_raw<W: Word>(
        channel: impl Peripheral<P = impl Channel> + 'a,
        request: Request,
        buf: *const [W],
        peri_addr: *mut W,
        options: TransferOptions,
    ) -> Self {
        into_ref!(channel);

        Self::new_inner(
            channel.map_into(),
            request,
            Dir::MemoryToPeripheral,
            peri_addr as *const u32,
            buf as *const W as *mut u32,
            buf.len(),
            true,
            W::size(),
            options,
        )
    }

    /// Create a new write DMA transfer (memory to peripheral), writing the same value repeatedly.
    pub unsafe fn new_write_repeated<W: Word>(
        channel: impl Peripheral<P = impl Channel> + 'a,
        request: Request,
        repeated: &'a W,
        count: usize,
        peri_addr: *mut W,
        options: TransferOptions,
    ) -> Self {
        into_ref!(channel);

        Self::new_inner(
            channel.map_into(),
            request,
            Dir::MemoryToPeripheral,
            peri_addr as *const u32,
            repeated as *const W as *mut u32,
            count,
            false,
            W::size(),
            options,
        )
    }

    unsafe fn new_inner(
        channel: PeripheralRef<'a, AnyChannel>,
        request: Request,
        dir: Dir,
        peri_addr: *const u32,
        mem_addr: *mut u32,
        mem_len: usize,
        incr_mem: bool,
        data_size: WordSize,
        _options: TransferOptions,
    ) -> Self {
        assert!(mem_len > 0 && mem_len <= 0xFFFF);

        let info = channel.info();
        let ch = info.dma.ch(info.num);

        // "Preceding reads and writes cannot be moved past subsequent writes."
        fence(Ordering::SeqCst);

        let this = Self { channel };

        // dmamuxはないっぽいな
        // #[cfg(dmamux)]
        // super::dmamux::configure_dmamux(&*this.channel, request);

        ch.cr().write(|w| w.set_reset(true));
        ch.fcr().write(|w| w.0 = 0xFFFF_FFFF); // clear all irqs
        ch.llr().write(|_| {}); // no linked list
        ch.tr1().write(|w| {
            w.set_sdw(data_size.into());
            w.set_ddw(data_size.into());
            w.set_sinc(dir == Dir::MemoryToPeripheral && incr_mem);
            w.set_dinc(dir == Dir::PeripheralToMemory && incr_mem);
        });
        ch.tr2().write(|w| {
            w.set_dreq(match dir {
                Dir::MemoryToPeripheral => vals::Dreq::DESTINATIONPERIPHERAL,
                Dir::PeripheralToMemory => vals::Dreq::SOURCEPERIPHERAL,
            });
            w.set_reqsel(request);
        });
        ch.br1().write(|w| {
            // BNDT is specified as bytes, not as number of transfers.
            w.set_bndt((mem_len * data_size.bytes()) as u16)
        });

        match dir {
            Dir::MemoryToPeripheral => {
                ch.sar().write_value(mem_addr as _);
                ch.dar().write_value(peri_addr as _);
            }
            Dir::PeripheralToMemory => {
                ch.sar().write_value(peri_addr as _);
                ch.dar().write_value(mem_addr as _);
            }
        }

        ch.cr().write(|w| {
            // Enable interrupts
            w.set_tcie(true);
            w.set_useie(true);
            w.set_dteie(true);
            w.set_suspie(true);

            // Start it
            w.set_en(true);
        });

        this
    }

    /// Request the transfer to stop.
    ///
    /// This doesn't immediately stop the transfer, you have to wait until [`is_running`](Self::is_running) returns false.
    pub fn request_stop(&mut self) {
        let info = self.channel.info();
        let ch = info.dma.ch(info.num);

        ch.cr().modify(|w| w.set_susp(true))
    }

    /// Return whether this transfer is still running.
    ///
    /// If this returns `false`, it can be because either the transfer finished, or
    /// it was requested to stop early with [`request_stop`](Self::request_stop).
    pub fn is_running(&mut self) -> bool {
        let info = self.channel.info();
        let ch = info.dma.ch(info.num);

        let sr = ch.sr().read();
        !sr.tcf() && !sr.suspf()
    }

    /// Gets the total remaining transfers for the channel
    /// Note: this will be zero for transfers that completed without cancellation.
    pub fn get_remaining_transfers(&self) -> u16 {
        let info = self.channel.info();
        let ch = info.dma.ch(info.num);

        ch.br1().read().bndt()
    }
}
}