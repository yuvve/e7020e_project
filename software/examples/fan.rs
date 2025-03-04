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
    systick_monotonic::*,
};

const TIMER_HZ: u32 = 1000;
const DUTY_TURNAROUND: u16 = 300;
const MAX_DUTY: u16 = 1000;
const DUTY_STEP: u16 = 10;
const TIMER_STEP_MS: u64 = 50;

#[rtic::app(device = nrf52833_hal::pac, dispatchers = [TIMER0])]
mod app {
    use super::*;

    type PWM = Pwm<PWM0>;

    #[monotonic(binds = SysTick, default = true)]
    type MyMono = Systick<TIMER_HZ>;

    #[shared]
    struct Shared {
        pwm: PWM,  // PWM is now shared
    }

    #[local]
    struct Local {
        duty: u16,
        increasing: bool,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        let mono = Systick::new(cx.core.SYST, 64_000_000);

        // LED setup
        let port0 = hal::gpio::p0::Parts::new(cx.device.P0);
        let led: Pin<Output<PushPull>> = port0.p0_09.into_push_pull_output(Level::Low).degrade();

        // NFC pin setup (unchanged)
        // ...

        // PWM setup
        let pwm = Pwm::new(cx.device.PWM0);
        pwm.set_prescaler(Prescaler::Div16);
        pwm.set_output_pin(Channel::C0, led);
        pwm.set_max_duty(MAX_DUTY);
        pwm.set_duty_off(Channel::C0, 0);
        pwm.enable();

        // Schedule tasks
        dim_led::spawn_after(TIMER_STEP_MS.millis()).unwrap();
        turn_off_led::spawn_after(2.secs()).unwrap();  // Changed from 5.secs() to 2.secs()

        (
            Shared { pwm },  // PWM moved to Shared
            Local { duty: 0, increasing: true },
            init::Monotonics(mono),
        )
    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        rtt_init_print!();
        rprintln!("idle");
        loop { asm::wfi(); }
    }

    #[task(shared = [pwm], local = [duty, increasing])]
    fn dim_led(cx: dim_led::Context) {
        let dim_led::SharedResources { mut pwm } = cx.shared;  // Declare `pwm` as mutable
        let dim_led::LocalResources { duty, increasing } = cx.local;

        // Lock the PWM for exclusive access
        pwm.lock(|pwm| {
            if !*increasing && *duty <= DUTY_STEP {
                *increasing = true;
            } else if *duty >= DUTY_TURNAROUND {
                *increasing = false;
            }

            if *increasing {
                pwm.set_duty_off(Channel::C0, 0);
                *duty += DUTY_STEP;
            } else {
                pwm.set_duty_off(Channel::C0, 0);
                *duty -= DUTY_STEP;
            }
        });

        rprintln!("dim_led: duty = {}", *duty);
        dim_led::spawn_after(TIMER_STEP_MS.millis()).unwrap();
    }

    #[task(shared = [pwm])]
    fn turn_off_led(mut cx: turn_off_led::Context) {
        cx.shared.pwm.lock(|pwm| {
            pwm.set_duty_off(Channel::C0, 0);  // Turn off LED
            rprintln!("LED turned off");
        });
    }
}