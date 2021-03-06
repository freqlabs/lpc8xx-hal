//! ADC example for lpc845

#![no_main]
#![no_std]

extern crate panic_halt;

use core::fmt::Write;
use nb::block;

use lpc8xx_hal::{
    cortex_m_rt::entry, delay::Delay, prelude::*,
    syscon::clock_source::AdcClock, usart, CorePeripherals, Peripherals,
};

#[entry]
fn main() -> ! {
    let cp = CorePeripherals::take().unwrap();
    let p = Peripherals::take().unwrap();

    let mut delay = Delay::new(cp.SYST);
    let swm = p.SWM.split();
    let mut syscon = p.SYSCON.split();

    let mut handle = swm.handle.enable(&mut syscon.handle); // SWM isn't enabled by default on LPC845.

    // Set baud rate to 115200 baud
    let clock_config = usart::Clock::new_with_baudrate(115200);

    let tx_pin = p.pins.pio0_25.into_swm_pin();
    let rx_pin = p.pins.pio0_24.into_swm_pin();

    let (u0_rxd, _) = swm.movable_functions.u0_rxd.assign(rx_pin, &mut handle);
    let (u0_txd, _) = swm.movable_functions.u0_txd.assign(tx_pin, &mut handle);

    let mut serial =
        p.USART0
            .enable(&clock_config, &mut syscon.handle, u0_rxd, u0_txd);

    let adc_clock = AdcClock::new_default();
    let mut adc = p.ADC.enable(&adc_clock, &mut syscon.handle);

    let (mut adc_pin, _) = swm
        .fixed_functions
        .adc_0
        .assign(p.pins.pio0_7.into_swm_pin(), &mut handle);

    loop {
        let adc_value =
            block! {adc.read(&mut adc_pin)}.expect("Read should never fail");
        write!(serial, "{}\n", adc_value).expect("Write should never fail");
        delay.delay_ms(100u8);
    }
}
