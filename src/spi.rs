//! API for the SPI peripherals
//!
//! # Example
//!
//! ``` no_run
//! use lpc8xx_hal::{
//!     prelude::*,
//!     Peripherals,
//!     syscon::clock_source::SpiClock,
//! };
//!
//! let mut p  = Peripherals::take().unwrap();
//! let mut swm = p.SWM.split();
//! let mut syscon = p.SYSCON.split();
//!
//! #[cfg(feature = "82x")]
//! let mut swm_handle = swm.handle;
//! #[cfg(feature = "845")]
//! let mut swm_handle = swm.handle.enable(&mut syscon.handle);
//!
//! let (spi0_sck, _) = swm.movable_functions.spi0_sck.assign(
//!     p.pins.pio0_13.into_swm_pin(),
//!     &mut swm_handle,
//! );
//! let (spi0_mosi, _) = swm
//!     .movable_functions
//!     .spi0_mosi
//!     .assign(p.pins.pio0_14.into_swm_pin(), &mut swm_handle);
//! let (spi0_miso, _) = swm
//!     .movable_functions
//!     .spi0_miso
//!     .assign(p.pins.pio0_15.into_swm_pin(), &mut swm_handle);
//!
//! #[cfg(feature = "82x")]
//! let spi_clock = SpiClock::new(0);
//! #[cfg(feature = "845")]
//! let spi_clock = SpiClock::new(&syscon.iosc, 0);
//!
//! // Enable SPI0
//! let mut spi = p.SPI0.enable(
//!     &spi_clock,
//!     &mut syscon.handle,
//!     embedded_hal::spi::MODE_0,
//!     spi0_sck,
//!     spi0_mosi,
//!     spi0_miso,
//! );
//!
//! let mut tx_data = [0x00, 0x01];
//! let rx_data = spi.transfer(&mut tx_data)
//!     .expect("Transfer shouldn't fail");
//! ```
//!
//! Please refer to the [examples in the repository] for more example code.
//!
//! [examples in the repository]: https://github.com/lpc-rs/lpc8xx-hal/tree/master/examples

use core::ops::Deref;

use embedded_hal::spi::{FullDuplex, Mode, Phase, Polarity};

use crate::{
    init_state, pac, pins,
    swm::{self, FunctionTrait},
    syscon::{
        self,
        clock_source::{PeripheralClock, SpiClock},
    },
};

/// Interface to a SPI peripheral
///
/// Controls the SPI. Use [`Peripherals`] to gain access to an instance of
/// this struct.
///
/// Please refer to the [module documentation] for more information.
///
/// # `embedded-hal` traits
///
/// - [`embedded_hal::spi::FullDuplex`] for asynchronous transfers
/// - [`embedded_hal::blocking::spi::Transfer`] for synchronous transfers
/// - [`embedded_hal::blocking::spi::Write`] for synchronous writes
///
/// [`Peripherals`]: ../struct.Peripherals.html
/// [module documentation]: index.html
/// [`embedded_hal::spi::FullDuplex`]: #impl-FullDuplex%3Cu8%3E
/// [`embedded_hal::blocking::spi::Transfer`]: #impl-Transfer%3CW%3E
/// [`embedded_hal::blocking::spi::Write`]: #impl-Write%3CW%3E
pub struct SPI<I, State = init_state::Enabled> {
    spi: I,
    _state: State,
}

impl<I> SPI<I, init_state::Disabled>
where
    I: Instance,
{
    pub(crate) fn new(spi: I) -> Self {
        Self {
            spi,
            _state: init_state::Disabled,
        }
    }

    /// Enable the SPI peripheral
    ///
    /// This method is only available, if `SPI` is in the [`Disabled`] state.
    /// Code that attempts to call this method when the peripheral is already
    /// enabled will not compile.
    ///
    /// Consumes this instance of `SPI` and returns another instance that has
    /// its `State` type parameter set to [`Enabled`].
    ///
    /// # Examples
    ///
    /// Please refer to the [module documentation] for a full example.
    ///
    /// [`Disabled`]: ../init_state/struct.Disabled.html
    /// [`Enabled`]: ../init_state/struct.Enabled.html
    /// [`BaudRate`]: struct.BaudRate.html
    /// [module documentation]: index.html
    pub fn enable<SckPin, MosiPin, MisoPin, CLOCK>(
        self,
        clock: &SpiClock<CLOCK>,
        syscon: &mut syscon::Handle,
        mode: Mode,
        _: swm::Function<I::Sck, swm::state::Assigned<SckPin>>,
        _: swm::Function<I::Mosi, swm::state::Assigned<MosiPin>>,
        _: swm::Function<I::Miso, swm::state::Assigned<MisoPin>>,
    ) -> SPI<I, init_state::Enabled>
    where
        SckPin: pins::Trait,
        MosiPin: pins::Trait,
        MisoPin: pins::Trait,
        I::Sck: FunctionTrait<SckPin>,
        I::Mosi: FunctionTrait<MosiPin>,
        I::Miso: FunctionTrait<MisoPin>,
        SpiClock<CLOCK>: PeripheralClock<I>,
    {
        syscon.enable_clock(&self.spi);

        clock.select_clock(syscon);

        self.spi
            .div
            .write(|w| unsafe { w.divval().bits(clock.divval) });

        self.spi.txctl.write(|w| {
            // 8 bit length
            unsafe { w.len().bits(7) }
        });

        self.spi.cfg.write(|w| {
            if mode.polarity == Polarity::IdleHigh {
                w.cpol().high();
            } else {
                w.cpol().low();
            }
            if mode.phase == Phase::CaptureOnFirstTransition {
                w.cpha().clear_bit();
            } else {
                w.cpha().set_bit();
            }
            w.enable().enabled();
            w.master().master_mode()
        });

        SPI {
            spi: self.spi,
            _state: init_state::Enabled(()),
        }
    }
}

impl<I> SPI<I, init_state::Enabled>
where
    I: Instance,
{
    /// Disable the SPI peripheral
    ///
    /// This method is only available, if `SPI` is in the [`Enabled`] state.
    /// Code that attempts to call this method when the peripheral is already
    /// disabled will not compile.
    ///
    /// Consumes this instance of `SPI` and returns another instance that has
    /// its `State` type parameter set to [`Disabled`].
    ///
    /// [`Enabled`]: ../init_state/struct.Enabled.html
    /// [`Disabled`]: ../init_state/struct.Disabled.html
    pub fn disable(
        self,
        syscon: &mut syscon::Handle,
    ) -> SPI<I, init_state::Disabled> {
        syscon.disable_clock(&self.spi);

        SPI {
            spi: self.spi,
            _state: init_state::Disabled,
        }
    }
}

impl<I, State> SPI<I, State> {
    /// Return the raw peripheral
    ///
    /// This method serves as an escape hatch from the HAL API. It returns the
    /// raw peripheral, allowing you to do whatever you want with it, without
    /// limitations imposed by the API.
    ///
    /// If you are using this method because a feature you need is missing from
    /// the HAL API, please [open an issue] or, if an issue for your feature
    /// request already exists, comment on the existing issue, so we can
    /// prioritize it accordingly.
    ///
    /// [open an issue]: https://github.com/lpc-rs/lpc8xx-hal/issues
    pub fn free(self) -> I {
        self.spi
    }
}

impl<I: Instance> FullDuplex<u8> for SPI<I> {
    type Error = ();

    fn read(&mut self) -> nb::Result<u8, Self::Error> {
        if self.spi.stat.read().rxrdy().bit_is_set() {
            Ok(self.spi.rxdat.read().rxdat().bits() as u8)
        } else {
            Err(nb::Error::WouldBlock)
        }
    }

    fn send(&mut self, word: u8) -> nb::Result<(), Self::Error> {
        if self.spi.stat.read().txrdy().bit_is_set() {
            self.spi
                .txdat
                .write(|w| unsafe { w.data().bits(word as u16) });
            Ok(())
        } else {
            Err(nb::Error::WouldBlock)
        }
    }
}

/// Internal trait for SPI peripherals
///
/// This trait is an internal implementation detail and should neither be
/// implemented nor used outside of LPC8xx HAL. Any changes to this trait won't
/// be considered breaking changes.
pub trait Instance:
    Deref<Target = pac::spi0::RegisterBlock>
    + syscon::ClockControl
    + syscon::ResetControl
{
    /// The movable function that needs to be assigned to this SPI's SCK pin
    type Sck;

    /// The movable function that needs to be assigned to this SPI's MOSI pin
    type Mosi;

    /// The movable function that needs to be assigned to this SPI's MISO pin
    type Miso;
}

impl Instance for pac::SPI0 {
    type Sck = swm::SPI0_SCK;
    type Mosi = swm::SPI0_MOSI;
    type Miso = swm::SPI0_MISO;
}

impl Instance for pac::SPI1 {
    type Sck = swm::SPI1_SCK;
    type Mosi = swm::SPI1_MOSI;
    type Miso = swm::SPI1_MISO;
}

impl<I: Instance> embedded_hal::blocking::spi::transfer::Default<u8>
    for SPI<I>
{
}

impl<I: Instance> embedded_hal::blocking::spi::write::Default<u8> for SPI<I> {}
