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
const SEQ_REFRESH: u32 = 500; // Periods per step
const MAX_DUTY: u16 = 10000;

pub static PWM_DUTY_CYCLE_SEQUENCE: [u16; 100] = [
    10000, 9990, 9980, 9970, 9960, 9950, 9940, 9930, 9920, 9910, 9900, 9890, 9880, 9870, 9860, 9850, 9840, 9830, 9820, 9810,
    9800, 9790, 9780, 9770, 9760, 9750, 9740, 9730, 9720, 9710, 9700, 9690, 9680, 9670, 9660, 9650, 9640, 9630, 9620, 9610,
    9600, 9590, 9580, 9570, 9560, 9550, 9540, 9530, 9520, 9510, 9500, 9490, 9480, 9470, 9460, 9450, 9440, 9430, 9420, 9410,
    9400, 9390, 9380, 9370, 9360, 9350, 9340, 9330, 9320, 9310, 9300, 9290, 9280, 9270, 9260, 9250, 9240, 9230, 9220, 9210,
    9200, 9190, 9180, 9170, 9160, 9150, 9140, 9130, 9120, 9110, 9100, 9090, 9080, 9070, 9060, 9050, 9040, 9030, 9020, 9010
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
            .set_max_duty(MAX_DUTY)
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
