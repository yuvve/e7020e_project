//! examples/led

#![no_main]
#![no_std]
#![deny(unsafe_code)]
//#![deny(warnings)]

use {
    cortex_m::asm, 
    hal::gpio::{Level, Output, Pin, PushPull}, 
    hal::pwm::*,

    nrf52833_hal as hal, 
    panic_rtt_target as _, 
    rtt_target::{rprintln, rtt_init_print}, 
    systick_monotonic::*
};

const TIMER_HZ: u32 = 1000; // 1000 Hz (1 ms granularity)
const SEQ_REFRESH: u32 = 2000; // Periods per step, 2 s per step

// 1000 is off, 0 is full brightness
pub static PWM_DUTY_CYCLE_SEQUENCE: [u16; 100] = [
    1000, 999, 998, 997, 996, 995, 994, 993, 992, 991, 990, 989, 988, 987, 986, 985, 984, 983, 982, 981,
    980, 979, 978, 977, 976, 975, 974, 973, 972, 971, 970, 969, 968, 967, 966, 965, 964, 963, 962, 961,
    960, 959, 958, 957, 956, 955, 954, 953, 952, 951, 950, 949, 948, 947, 946, 945, 944, 943, 942, 941,
    940, 939, 938, 937, 936, 935, 934, 933, 932, 931, 930, 929, 928, 927, 926, 925, 924, 923, 922, 921,
    920, 919, 918, 917, 916, 915, 914, 913, 912, 911, 910, 909, 908, 907, 906, 905, 904, 903, 902, 901
];

#[rtic::app(device = nrf52833_hal::pac, dispatchers= [TIMER0])]
mod app {
    use super::*;

    #[monotonic(binds = SysTick, default = true)]
    type MyMono = Systick<TIMER_HZ>;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {}

    #[init(local = [ 
        BUF0: [u16; 100] = PWM_DUTY_CYCLE_SEQUENCE,
        BUF1: [u16; 100] = PWM_DUTY_CYCLE_SEQUENCE,
    ])]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        let BUF0 = cx.local.BUF0;
        let BUF1 = cx.local.BUF1;
        // Initialize the monotonic (core clock at 64 MHz)
        let mono = Systick::new(cx.core.SYST, 64_000_000);

        // LED
        let port0 = hal::gpio::p0::Parts::new(cx.device.P0);
        let led: Pin<Output<PushPull>> = port0.p0_09.into_push_pull_output(Level::Low).degrade();

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

        pwm.set_prescaler(Prescaler::Div16)
            .set_max_duty(1000)
            .set_output_pin(Channel::C0, led)
            .set_counter_mode(CounterMode::Up)
            .set_load_mode(LoadMode::Common)
            .set_step_mode(StepMode::Auto)
            .set_seq_refresh(Seq::Seq0, SEQ_REFRESH)
            .set_seq_refresh(Seq::Seq1, SEQ_REFRESH)
            .one_shot()
            .enable();
        pwm.load(Some(BUF0), Some(BUF1), true).ok();

        (Shared {}, Local {}, init::Monotonics(mono))
    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        rtt_init_print!();
        rprintln!("idle");
        loop {
            asm::wfi();
        }
    }
}
