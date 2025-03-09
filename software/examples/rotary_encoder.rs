//! examples/rotary_encoder

#![no_main]
#![no_std]
#![deny(unsafe_code)]
#![deny(warnings)]

use {
    cortex_m::asm, 
    hal::gpio::{Level, Output, Pin, PushPull}, 
    hal::pwm::*,
    hal::qdec::*,
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

const DECREASE: i16 = 1;
const INCREASE: i16 = -1;
const OFF: i16 = 0;
const ROTARY_ENCODER_THRESHOLD_SEC: f32 = 0.08;

#[rtic::app(device = nrf52833_hal::pac, dispatchers= [TIMER0])]
mod app {
    use super::*;

    type PWM = Pwm<PWM0>;
    type GPIOTE = hal::gpiote::Gpiote;

    #[monotonic(binds = SysTick, default = true)]
    type MyMono = Systick<TIMER_HZ>;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        qdec: Qdec,
        pwm: PWM,
        duty: u16,
        gpiote: GPIOTE
    }

    #[init]
    fn init(mut cx: init::Context) -> (Shared, Local, init::Monotonics) {
        // Initialize the monotonic (core clock at 64 MHz)
        let mono = Systick::new(cx.core.SYST, 64_000_000);

        // LED
        let port0 = hal::gpio::p0::Parts::new(cx.device.P0);
        let led: Pin<Output<PushPull>> = port0.p0_09.into_push_pull_output(Level::Low).degrade();

        // Rotary encoder
        let rotary_encoder_pins =hal::qdec::Pins {
            a: port0.p0_30.into_pullup_input().degrade(),
            b: port0.p0_29.into_pullup_input().degrade(),
            led: None,
        };

        let qdec = Qdec::new(cx.device.QDEC, rotary_encoder_pins, SamplePeriod::_2048us);
        qdec.enable_interrupt(NumSamples::_1smpl)
            .debounce(true)
            .enable();

        // Rotary encoder switch
        let rotary_switch = port0.p0_28.into_pullup_input().degrade();
        let gpiote = hal::gpiote::Gpiote::new(cx.device.GPIOTE);
        gpiote.channel0().input_pin(&rotary_switch)
            .hi_to_lo()
            .enable_interrupt();

        // Check if UICR is set correctly
        let check_uicr_set = cx.device.UICR.nfcpins.read().protect().is_disabled();

        // Set NFC pins to normal GPIO
        if !check_uicr_set {
            cx.device.NVMC.config.write(|w| w.wen().wen());
            while cx.device.NVMC.ready.read().ready().is_busy() {}
            
            cx.device.UICR.nfcpins.write(|w| w.protect().disabled());
            while cx.device.NVMC.ready.read().ready().is_busy() {}

            cx.device.NVMC.config.write(|w| w.wen().ren());
            while cx.device.NVMC.ready.read().ready().is_busy() {}

            // Changes to UICR require a reset to take effect
            cortex_m::peripheral::SCB::sys_reset();
        }

        // PWM
        let pwm = Pwm::new(cx.device.PWM0);
        let duty = 0;

        pwm.set_prescaler(Prescaler::Div16);    // 1 kHz PWM frequency
        pwm.set_output_pin(Channel::C0, led);
        pwm.set_max_duty(MAX_DUTY);             
        pwm.set_duty_off(Channel::C0, duty);    // start at 0% duty cycle
        pwm.enable();

        // Enable cycle counter
        cx.core.DCB.enable_trace();
        cx.core.DWT.enable_cycle_counter();

        (
            Shared {}, 
            Local {
                qdec,
                pwm, 
                duty,
                gpiote
            }, 
            init::Monotonics(mono)
        )
    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        rtt_init_print!();
        rprintln!("idle");
        loop {
            asm::wfi();
        }
    }

    #[task(binds = QDEC, local = [qdec, compare_cycle: u32 = 0, prev_direction: i16 = 0])]
    fn qdec_interrupt(cx: qdec_interrupt::Context) {
        let qdec = cx.local.qdec;
        qdec.reset_events();
        let direction = qdec.read();
        *cx.local.prev_direction = direction;

        let now = cortex_m::peripheral::DWT::cycle_count();
        let elapsed_cycles = now.wrapping_sub(*cx.local.compare_cycle);
        let elapsed_time = elapsed_cycles as f32 / 64_000_000.0;

        // Filter out debounce noise
        if !(elapsed_time <= ROTARY_ENCODER_THRESHOLD_SEC) {
            *cx.local.compare_cycle = now;

            match direction {
                DECREASE => {
                    rprintln!("qdec_interrupt: LEFT");
                    set_led::spawn(DECREASE).unwrap()
                },
                INCREASE => {
                    rprintln!("qdec_interrupt: RIGHT");
                    set_led::spawn(INCREASE).unwrap()
                },
                _ => {}
            }
        }
    }

    #[task(local = [pwm, duty])]
    fn set_led(cx: set_led::Context, value: i16) {
        let pwm = cx.local.pwm;
        let duty = *cx.local.duty;

        match value {
            INCREASE => {
                if duty + DUTY_STEP <= DUTY_TURNAROUND {
                    pwm.set_duty_off(Channel::C0, duty + DUTY_STEP);
                    *cx.local.duty += DUTY_STEP;
                }
            },
            DECREASE => {
                if duty >= DUTY_STEP {
                    pwm.set_duty_off(Channel::C0, duty - DUTY_STEP);
                    *cx.local.duty -= DUTY_STEP;
                }
            },
            OFF => {
                pwm.set_duty_off(Channel::C0, 0);
                *cx.local.duty = 0;
            },
            _ => {}
        }

        rprintln!("set_led: duty = {}", cx.local.duty);
    }

    #[task(binds = GPIOTE, local = [gpiote])]
    fn gpiote_interrupt(cx: gpiote_interrupt::Context) {
        rprintln!("gpiote_interrupt");
        
        let gpiote = cx.local.gpiote;
        gpiote.channel0().reset_events();

        set_led::spawn(OFF).unwrap();
    }
}
