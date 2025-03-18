use {
    crate::app::*,
    hal::{
        gpio::{Output, Pin, PushPull},
        pac::PWM0,
        pwm::*,
    },
    nrf52833_hal::{self as hal},
};

// 10000 is off, 0 is 100% duty cycle
// This needs to be tested and adjusted for all the components
pub static PWM_DUTY_CYCLE_SEQUENCE: [u16; 100] = [
    10000, 9990, 9980, 9970, 9960, 9950, 9940, 9930, 9920, 9910, 9900, 9890, 9880, 9870, 9860,
    9850, 9840, 9830, 9820, 9810, 9800, 9790, 9780, 9770, 9760, 9750, 9740, 9730, 9720, 9710, 9700,
    9690, 9680, 9670, 9660, 9650, 9640, 9630, 9620, 9610, 9600, 9590, 9580, 9570, 9560, 9550, 9540,
    9530, 9520, 9510, 9500, 9490, 9480, 9470, 9460, 9450, 9440, 9430, 9420, 9410, 9400, 9390, 9380,
    9370, 9360, 9350, 9340, 9330, 9320, 9310, 9300, 9290, 9280, 9270, 9260, 9250, 9240, 9230, 9220,
    9210, 9200, 9190, 9180, 9170, 9160, 9150, 9140, 9130, 9120, 9110, 9100, 9090, 9080, 9070, 9060,
    9050, 9040, 9030, 9020, 9010,
];

const SEQ_REFRESH: u32 = 1000; // Periods per step
const MAX_DUTY: u16 = 10000;

pub type SeqBuffer = &'static mut [u16; 100];
pub type Pwm0 = Option<PwmSeq<PWM0, SeqBuffer, SeqBuffer>>;

pub(crate) fn init(
    pwm: PWM0,
    led_pin: Pin<Output<PushPull>>,
    amp_fan_hum_pin: Pin<Output<PushPull>>,
    haptic_pin: Pin<Output<PushPull>>,
) -> Pwm<PWM0> {
    let pwm = hal::pwm::Pwm::new(pwm);
    pwm.set_prescaler(Prescaler::Div16)
        .set_max_duty(MAX_DUTY)
        .set_output_pin(Channel::C0, led_pin)
        .set_output_pin(Channel::C1, amp_fan_hum_pin)
        .set_output_pin(Channel::C2, haptic_pin)
        .set_counter_mode(CounterMode::Up)
        .set_load_mode(LoadMode::Common)
        .set_step_mode(StepMode::Auto)
        .set_seq_refresh(Seq::Seq0, SEQ_REFRESH)
        .set_seq_refresh(Seq::Seq1, SEQ_REFRESH)
        .one_shot()
        .enable();
    pwm
}

pub(crate) fn start(cx: start_pwm::Context) {
    let pwm = cx.shared.pwm.as_ref().unwrap();
    pwm.start_seq(Seq::Seq0);
}

pub(crate) fn stop(cx: stop_pwm::Context) {
    let pwm = cx.shared.pwm.as_ref().unwrap();
    pwm.stop();
}
