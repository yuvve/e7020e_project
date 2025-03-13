//! examples/led

#![no_main]
#![no_std]
#![deny(unsafe_code)]
#![deny(warnings)]

use {
    cortex_m::asm, 
    hal::gpio::{Level, Output, Pin, PushPull}, 
    hal::pwm::*,
    hal::pac::PWM0,

    nrf52833_hal as hal, 
    panic_rtt_target as _, 
    rtt_target::{rprintln, rtt_init_print}, 
    systick_monotonic::*
};

const TIMER_HZ: u32 = 1000; // 1000 Hz (1 ms granularity)
const DUTY_TURNAROUND: u16 = 300;
const MAX_DUTY: u16 = 1000;  // 1 kHz duty resolution
const DUTY_STEP: u16 = 10;
const TIMER_STEP_MS: u64 = 50;

#[rtic::app(device = nrf52833_hal::pac, dispatchers= [TIMER0])]
mod app {
    use super::*;

    type PWM = Pwm<PWM0>;

    #[monotonic(binds = SysTick, default = true)]
    type MyMono = Systick<TIMER_HZ>;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        pwm: PWM,
        duty: u16,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        // Initialize the monotonic (core clock at 64 MHz)
        let mono = Systick::new(cx.core.SYST, 64_000_000);

        // LED
        let port0 = hal::gpio::p0::Parts::new(cx.device.P0);
        let haptic: Pin<Output<PushPull>> = port0.p0_20.into_push_pull_output(Level::Low).degrade();

        // PWM
        let pwm = Pwm::new(cx.device.PWM0);
        let duty = 0;

        pwm.set_prescaler(Prescaler::Div16);    // 1 kHz PWM frequency
        pwm.set_output_pin(Channel::C0, haptic);
        pwm.set_max_duty(MAX_DUTY);             
        pwm.set_duty_off(Channel::C0, duty);    // start at 0% duty cycle
        pwm.enable();

        dim_haptic::spawn_after(TIMER_STEP_MS.millis()).unwrap();
        (Shared {}, Local {pwm, duty}, init::Monotonics(mono))
    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        rtt_init_print!();
        rprintln!("idle");
        loop {
            asm::wfi();
        }
    }

    #[task(local = [pwm, duty, increasing:bool = true])]
    fn dim_haptic(cx: dim_haptic::Context) {
        let pwm = cx.local.pwm;
        let duty = *cx.local.duty;
        let increasing = *cx.local.increasing;
        
        if !increasing && duty <= DUTY_STEP {
            *cx.local.increasing = true;
        } else if duty >= DUTY_TURNAROUND {
            *cx.local.increasing = false;
        }

        if increasing {
            pwm.set_duty_off(Channel::C0, duty);
            *cx.local.duty += DUTY_STEP;
        } else {
            pwm.set_duty_off(Channel::C0, duty);
            *cx.local.duty -= DUTY_STEP;
        }

        rprintln!("dim_led: duty = {}", duty);
        dim_haptic::spawn_after(TIMER_STEP_MS.millis()).unwrap();
    }
}