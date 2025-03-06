//! examples/thermistor

#![no_main]
#![no_std]
#![deny(unsafe_code)]
//#![deny(warnings)]

use {
    cortex_m::asm, 
    hal::gpio::{Level, Output, Pin, PushPull, p0::P0_03, Disconnected}, 
    hal::pwm::*,
    hal::pac::PWM0,
    hal::saadc::*,

    libm::logf,
    nrf52833_hal as hal, 
    panic_rtt_target as _, 
    rtt_target::{rprintln, rtt_init_print}, 
    systick_monotonic::*
};

const TIMER_HZ: u32 = 1000; // 1000 Hz (1 ms granularity)
const B: f32 = 3950.0;  // From the datasheet
const R_25: f32 = 10000.0;
const T_25: f32 = 298.15; // 25 C in Kelvin
const VDD: f32 = 2.9; // Measure the actual value of VDD
const ADC_MAX: f32 = 4095.0;

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
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        // Initialize the monotonic (core clock at 64 MHz)
        let mono = Systick::new(cx.core.SYST, 64_000_000);
        let port0 = hal::gpio::p0::Parts::new(cx.device.P0);

        let  saadc_config = SaadcConfig { 
            resolution: Resolution::_12BIT,
            oversample: Oversample::OVER8X,
            ..SaadcConfig::default()
        };
        let saadc = Saadc::new(cx.device.SAADC, saadc_config);
        let saadc_pin = port0.p0_03;

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
    fn read_thermistor(_cx: read_thermistor::Context) {
        rprintln!("read_thermistor");
        let saadc = _cx.local.saadc;
        let saadc_pin = _cx.local.saadc_pin;
        let adc_value = saadc.read_channel(saadc_pin).unwrap();

        let temperature = calculate_temperature(adc_value);
        rprintln!("Temperature: {:.2} C", temperature);

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
