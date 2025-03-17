use {
    hal::{
        gpio::{Output, Pin, PushPull}, pac::PWM0, pwm::*
    },
    nrf52833_hal::{self as hal}, 
    crate::app::*,
};

// 1000 is off, 0 is full brightness
pub static PWM_DUTY_CYCLE_SEQUENCE: [u16; 100] = [
    1000, 999, 998, 997, 996, 995, 994, 993, 992, 991, 990, 989, 988, 987, 986, 985, 984, 983, 982, 981,
    980, 979, 978, 977, 976, 975, 974, 973, 972, 971, 970, 969, 968, 967, 966, 965, 964, 963, 962, 961,
    960, 959, 958, 957, 956, 955, 954, 953, 952, 951, 950, 949, 948, 947, 946, 945, 944, 943, 942, 941,
    940, 939, 938, 937, 936, 935, 934, 933, 932, 931, 930, 929, 928, 927, 926, 925, 924, 923, 922, 921,
    920, 919, 918, 917, 916, 915, 914, 913, 912, 911, 910, 909, 908, 907, 906, 905, 904, 903, 902, 901
];

const SEQ_REFRESH: u32 = 1000; // Periods per step

pub(crate) fn init(pwm: Pwm<PWM0>, led_pin: Pin<Output<PushPull>>) -> Pwm<PWM0> {
        pwm.set_prescaler(Prescaler::Div16)
            .set_max_duty(1000)
            .set_output_pin(Channel::C0, led_pin)
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
