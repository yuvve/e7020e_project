//! examples/thermistor

#![no_main]
#![no_std]
#![deny(unsafe_code)]
#![deny(warnings)]

use {
    cortex_m::asm, 
    hal::gpio::{p0::P0_03, Disconnected}, 
    hal::saadc::*,

    libm::logf,
    nrf52833_hal as hal, 
    panic_rtt_target as _, 
    rtt_target::{rprintln, rtt_init_print}, 
    systick_monotonic::*
};

const TIMER_HZ: u32 = 1000;     // 1000 Hz (1 ms granularity)
const B: f32 = 3950.0;          // From the datasheet
const R_25: f32 = 10000.0;      // 10k Ohm at 25 C
const T_25: f32 = 298.15;       // 25 C in Kelvin
const VDD: f32 = 2.8;           // Measured VDD
const ADC_MAX: f32 = 4095.0;    // 12-bit ADC max value

#[rtic::app(device = nrf52833_hal::pac, dispatchers= [TIMER0])]
mod app {
    use super::*;

    #[monotonic(binds = SysTick, default = true)]
    type MyMono = Systick<TIMER_HZ>;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        saadc: Saadc,
        saadc_pin: P0_03<Disconnected>,
    }

    #[init]
    fn init(mut cx: init::Context) -> (Shared, Local, init::Monotonics) {
        // Initialize the monotonic (core clock at 64 MHz)
        let mono = Systick::new(cx.core.SYST, 64_000_000);
        let port0 = hal::gpio::p0::Parts::new(cx.device.P0);

        let  saadc_config = SaadcConfig { 
            resolution: Resolution::_12BIT,
            oversample: Oversample::BYPASS,
            ..SaadcConfig::default()
        };
        let saadc = Saadc::new(cx.device.SAADC, saadc_config);
        let saadc_pin = port0.p0_03;

        // Enable cycle counter
        cx.core.DCB.enable_trace();
        cx.core.DWT.enable_cycle_counter();

        read_thermistor::spawn().unwrap();
        (Shared {}, Local {saadc, saadc_pin}, init::Monotonics(mono))
    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        rtt_init_print!();
        rprintln!("idle");
        loop {
            asm::wfi();
        }
    }

    #[task(local = [saadc, saadc_pin])]
    fn read_thermistor(cx: read_thermistor::Context) {
        rprintln!("read_thermistor");
        let saadc = cx.local.saadc;
        let saadc_pin = cx.local.saadc_pin;
        let before = cortex_m::peripheral::DWT::cycle_count();
        
        // NOTE: This is a blocking call, we should measure it
        let adc_value = saadc.read_channel(saadc_pin).unwrap();
        let after = cortex_m::peripheral::DWT::cycle_count();
        let elapsed_cycles = after.wrapping_sub(before);
        let elapsed_us = elapsed_cycles / 64; // 64 cycles per microsecond
        rprintln!("elapsed_us: {}", elapsed_us);
        rprintln!("elapsed_cycles: {}", elapsed_cycles);
        

        let temperature = calculate_temperature(adc_value);
        rprintln!("Temperature: {:.1} C", temperature);

        read_thermistor::spawn_after(1000_u64.millis()).unwrap();
    }  

    fn calculate_temperature(adc_value: i16) -> f32 {
        // Convert ADC value to voltage
        let v_adc = (adc_value as f32) * VDD / ADC_MAX;
        
        // Calculate using Beta formula
        let r_t = R_25 * (v_adc / (VDD - v_adc));
        let ln_r = logf(r_t / R_25);
        let b_t = B / T_25;
        let temperature = B / (ln_r + b_t);
        temperature - 273.15 // Convert to Celsius
    }
    
}
