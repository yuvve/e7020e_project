#![no_main]
#![no_std]
#![deny(unsafe_code)]
#![deny(warnings)]

use {
    cortex_m::asm,
    nrf52833_hal as hal,
    hal::pac::PWM0,
    hal::pwm::{Channel, Prescaler, Pwm},
    hal::gpio::{Level, Output, Pin, PushPull},
    panic_rtt_target as _,
    rtt_target::{rprintln, rtt_init_print},
    systick_monotonic::*,
};

/// Rated max speed is ~7300 RPM (per datasheet).
/// We'll treat that as 100%.
const FAN_MAX_RPM: u32 = 7300;
/// This is the PWM's top count. Control amount of steps
const MAX_DUTY_TICKS: u16 = 3000;
/// Delay (ms) between each ramp step
const RAMP_STEP_MS: u64 = 100;
/// How many % we go up each step
const STEP_PERCENT: u8 = 1;
/// If timer_value >= TURN_OFF_THRESH => fan is turned OFF
const TURN_OFF_THRESH: u32 = 1000;
/// Delay (ms) between each timer step
const FAN_RUNNING_MS: u64 = 100;

#[rtic::app(device = nrf52833_hal::pac, dispatchers=[TIMER0])]
mod app {
    use super::*;

    #[monotonic(binds = SysTick, default = true)]
    type MyMono = Systick<1000>;

    #[shared]
    struct Shared {
        pwm: Pwm<PWM0>,
        timer_value: u32,
    }

    #[local]
    struct Local {
        current_percent: u8,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        let mono = Systick::new(cx.core.SYST, 64_000_000);

        rtt_init_print!();
        rprintln!("init");

        // Configure GPIO pin P0.10 for fan
        let port0 = hal::gpio::p0::Parts::new(cx.device.P0);
        let fan_pin: Pin<Output<PushPull>> =
            port0.p0_10.into_push_pull_output(Level::Low).degrade();

        // Configure PWM
        let pwm = Pwm::new(cx.device.PWM0);
        pwm.set_prescaler(Prescaler::Div16); // => ~1 kHz if 16 MHz base
        pwm.set_output_pin(Channel::C0, fan_pin);
        pwm.set_max_duty(MAX_DUTY_TICKS);
        pwm.set_duty_off(Channel::C0, 0);
        pwm.enable();

        // Kick off the ramp
        ramp_up::spawn_after(RAMP_STEP_MS.millis()).unwrap();

        // Return shared/local resources & monotonic
        (
            Shared {
                pwm,
                timer_value: 0,
            },
            Local {
                current_percent: 0,
            },
            init::Monotonics(mono),
        )
    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        loop {
            asm::wfi();
        }
    }

    /// Ramps the fan from 0% to 100%.  
    /// Once at 100%, spawns `fan_running` to check timer for turn-off condition.
    #[task(shared = [pwm, timer_value], local = [current_percent])]
    fn ramp_up(mut cx: ramp_up::Context) {
        let percent = cx.local.current_percent;

        if *percent < 100 {
            *percent += STEP_PERCENT;
            if *percent > 100 {
                *percent = 100;
            }

            let duty_val = percent_to_duty(*percent);

            // Lock PWM to set new duty
            cx.shared.pwm.lock(|pwm| {
                pwm.set_duty_off(Channel::C0, duty_val);
            });
            let approx_rpm = percent_to_rpm(*percent);

            rprintln!(
                "Fan speed: {}% duty={} -> ~{} RPM",
                *percent,
                duty_val,
                approx_rpm
            );

            // Re-spawn until we hit 100%
            if *percent < 100 {
                ramp_up::spawn_after(RAMP_STEP_MS.millis()).unwrap();
            } else {
                rprintln!("Reached full speed! Starting fan_running loop...");
                fan_running::spawn_after(FAN_RUNNING_MS.millis()).unwrap();
            }
        }
    }

    /// Loops while fan is at 100% and checks timer_value for turn-off.
    #[task(shared = [pwm, timer_value])]
    fn fan_running(mut cx: fan_running::Context) {
        let mut turn_off = false;
        cx.shared.timer_value.lock(|timer_val| {
            *timer_val += 1;
            if *timer_val >= TURN_OFF_THRESH {
                rprintln!("Turning off");
                turn_off = true;
            }
        });

        if turn_off {
            cx.shared.pwm.lock(|pwm| {
                pwm.set_duty_off(Channel::C0, 0);
            });
        } else {
            cx.shared.timer_value.lock(|val| {
            rprintln!(
                "Fan at 100%. timer_value={} threshold={}",                    
                *val,
                TURN_OFF_THRESH
            );
        });
            fan_running::spawn_after(FAN_RUNNING_MS.millis()).unwrap();
        }
    }
}

/// Convert a 0–100% speed to a PWM duty register value
fn percent_to_duty(percent: u8) -> u16 {
    let pct = core::cmp::min(percent, 100) as u32;
    (pct * (MAX_DUTY_TICKS as u32) / 100) as u16
}

/// Convert a 0–100% speed to an approximate RPM based on FAN_MAX_RPM
fn percent_to_rpm(percent: u8) -> u32 {
    let pct = core::cmp::min(percent, 100) as u32;
    (pct * FAN_MAX_RPM) / 100
}