#![no_main]
#![no_std]
#![deny(unsafe_code)]
#![deny(warnings)]

use {
    core::option::Option,
    hal::{
        gpio::Level,
        pac::PWM0,
        pwm::*,
    },
    nrf52833_hal as hal,
    cortex_m::asm,
    rtt_target::{rprintln, rtt_init_print},
    systick_monotonic::*,
    panic_rtt_target as _,
};

// 10000 is off, 0 is 100% duty cycle
// This needs to be tested and adjusted for all the components
static LED_SEQUENCE: [u16; 100] = [
    10000, 9990, 9980, 9970, 9960, 9950, 9940, 9930, 9920, 9910, 9900, 9890, 9880, 9870, 9860,
    9850, 9840, 9830, 9820, 9810, 9800, 9790, 9780, 9770, 9760, 9750, 9740, 9730, 9720, 9710, 9700,
    9690, 9680, 9670, 9660, 9650, 9640, 9630, 9620, 9610, 9600, 9590, 9580, 9570, 9560, 9550, 9540,
    9530, 9520, 9510, 9500, 9490, 9480, 9470, 9460, 9450, 9440, 9430, 9420, 9410, 9400, 9390, 9380,
    9370, 9360, 9350, 9340, 9330, 9320, 9310, 9300, 9290, 9280, 9270, 9260, 9250, 9240, 9230, 9220,
    9210, 9200, 9190, 9180, 9170, 9160, 9150, 9140, 9130, 9120, 9110, 9100, 9090, 9080, 9070, 9060,
    9050, 9040, 9030, 9020, 9010,
];

static AMP_FAN_HUM_SEQUENCE: [u16; 100] = [
    5100, 5099, 5098, 5097, 5096, 5095, 5094, 5093, 5092, 5091, 5090, 5089, 5088, 5087, 5086,
    5085, 5084, 5083, 5082, 5081, 5080, 5079, 5078, 5077, 5076, 5075, 5074, 5073, 5072, 5071, 5070,
    5069, 5068, 5067, 5066, 5065, 5064, 5063, 5062, 5061, 5060, 5059, 5058, 5057, 5056, 5055, 5054,
    5053, 5052, 5051, 5050, 5049, 5048, 5047, 5046, 5045, 5044, 5043, 5042, 5041, 5040, 5039, 5038,
    5037, 5036, 5035, 5034, 5033, 5032, 5031, 5030, 5029, 5028, 5027, 5026, 5025, 5024, 5023, 5022,
    5021, 5020, 5019, 5018, 5017, 5016, 5015, 5014, 5013, 5012, 5011, 5010, 5009, 5008, 5007, 5006,
    5005, 5004, 5003, 5002, 5001,
];

static HAPTIC_SEQUENCE: [u16; 100] = [
    5100, 5099, 5098, 5097, 5096, 5095, 5094, 5093, 5092, 5091, 5090, 5089, 5088, 5087, 5086,
    5085, 5084, 5083, 5082, 5081, 5080, 5079, 5078, 5077, 5076, 5075, 5074, 5073, 5072, 5071, 5070,
    5069, 5068, 5067, 5066, 5065, 5064, 5063, 5062, 5061, 5060, 5059, 5058, 5057, 5056, 5055, 5054,
    5053, 5052, 5051, 5050, 5049, 5048, 5047, 5046, 5045, 5044, 5043, 5042, 5041, 5040, 5039, 5038,
    5037, 5036, 5035, 5034, 5033, 5032, 5031, 5030, 5029, 5028, 5027, 5026, 5025, 5024, 5023, 5022,
    5021, 5020, 5019, 5018, 5017, 5016, 5015, 5014, 5013, 5012, 5011, 5010, 5009, 5008, 5007, 5006,
    5005, 5004, 5003, 5002, 5001,
];

static CH3_SEQUENCE: [u16; 100] = [0u16; 100];

const SEQ_REFRESH: u32 = 9; // Extra periods per step
const MAX_DUTY: u16 = 10000;

const TIMER_HZ: u32 = 1000; // 1000 Hz (1 ms granularity)

pub type SeqBuffer = &'static mut [u16; 400];
pub type Pwm0 = Option<PwmSeq<PWM0, SeqBuffer, SeqBuffer>>;


#[rtic::app(device = nrf52833_hal::pac, dispatchers=[TIMER0])]
mod app {
    use super::*;

    #[monotonic(binds = SysTick, default = true)]
    type MyMono = Systick<TIMER_HZ>;


    #[shared]
    struct Shared {
        #[lock_free]
        pwm: Pwm0,
    }

    #[local]
    struct Local {
    }

    #[init(local = [
        SEQBUF0: [u16; 400] = [0u16; 400],
        SEQBUF1: [u16; 400] = [0u16; 400]
    ])]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        let SEQBUF0 = cx.local.SEQBUF0;
        let SEQBUF1 = cx.local.SEQBUF1;
        let mono = Systick::new(cx.core.SYST, 64_000_000);

        rtt_init_print!();
        rprintln!("init");

        // Configure GPIO pin P0.10 for fan
        let port0 = hal::gpio::p0::Parts::new(cx.device.P0);
        let led = port0.p0_09.into_push_pull_output(Level::Low).degrade();
        let amp_fan_hum = port0.p0_10.into_push_pull_output(Level::Low).degrade();
        let haptic = port0.p0_20.into_push_pull_output(Level::Low).degrade();

        // Configure PWM
        let pwm = Pwm::new(cx.device.PWM0);
        pwm.set_prescaler(Prescaler::Div16)
        .set_max_duty(MAX_DUTY)
        .set_output_pin(Channel::C0, led)
        .set_output_pin(Channel::C1, amp_fan_hum)
        .set_output_pin(Channel::C2, haptic)
        .set_counter_mode(CounterMode::Up)
        .set_load_mode(LoadMode::Individual)
        .set_step_mode(StepMode::Auto)
        .set_seq_refresh(Seq::Seq0, SEQ_REFRESH)
        .set_seq_refresh(Seq::Seq1, SEQ_REFRESH)
        .one_shot()
        .enable_interrupt(PwmEvent::PwmPeriodEnd)
        .enable();
        let pwm = pwm.load(Some(SEQBUF0), Some(SEQBUF1), false).ok();
        
        load_pwm_sequence::spawn().ok();
        start_pwm::spawn().ok();
        (
            Shared {
                pwm,
            },
            Local {
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

    #[task(binds = PWM0,  shared = [pwm], local = [seq_count: u32 = 0, period_count: u32 = 0])]
    fn pwm0(cx: pwm0::Context) {
        let pwm = cx.shared.pwm.as_ref().unwrap();
        pwm.reset_event(PwmEvent::PwmPeriodEnd);


        *cx.local.period_count = *cx.local.period_count + 1;
        if *cx.local.period_count % 1000 == 0 {
            *cx.local.seq_count = *cx.local.seq_count + 1;
            rprintln!("Next sequence, seq_count: {}", *cx.local.seq_count);
            rprintln!("LED: {}, AMP_FAN_HUM: {}, HAPTIC: {}", LED_SEQUENCE[*cx.local.seq_count as usize], AMP_FAN_HUM_SEQUENCE[*cx.local.seq_count as usize], HAPTIC_SEQUENCE[*cx.local.seq_count as usize]);
        }
    }


    #[task(shared = [pwm])]
    fn load_pwm_sequence(cx: load_pwm_sequence::Context) {
        let (buf0, buf1, pwm) = cx.shared.pwm.take().unwrap().split();
        let seqbuf0 = buf0.unwrap();
        let seqbuf1 = buf1.unwrap();

        for i in 0..100 {
            seqbuf0[i * 4] = LED_SEQUENCE[i];
            seqbuf0[i * 4 + 1] = AMP_FAN_HUM_SEQUENCE[i];
            seqbuf0[i * 4 + 2] = HAPTIC_SEQUENCE[i];
            seqbuf0[i * 4 + 3] = CH3_SEQUENCE[i];
        }
        seqbuf1.copy_from_slice(seqbuf0);
        let pwm = pwm.load(Some(seqbuf0), Some(seqbuf1), false).ok();
        *cx.shared.pwm = pwm;
        rprintln!("Loaded PWM sequence");
    }

    #[task(shared = [pwm])]
    fn start_pwm(cx: start_pwm::Context) {
        let pwm = cx.shared.pwm.as_ref().unwrap();
        pwm.start_seq(Seq::Seq0);
        rprintln!("Started PWM");
    }
}




