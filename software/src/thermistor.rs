const B: f32 = 3950.0;          // From the datasheet
const R_25: f32 = 10000.0;      // 10k Ohm at 25 C
const T_25: f32 = 298.15;       // 25 C in Kelvin
const VDD: f32 = 2.8;           // Measured VDD
const ADC_MAX: f32 = 4095.0;    // 12-bit ADC max value
const UPPER_LIMIT: f32 = 40.0;  // Upper limit for temperature
const LOWER_LIMIT: f32 = 0.0;   // Lower limit for temperature

use {
    crate::app::*,
    hal::saadc::*, 
    nrf52833_hal::{self as hal}, 
    libm::logf,
    rtic::Mutex,
};

pub(crate) fn init(saadc: hal::pac::SAADC) -> Saadc {
    let  saadc_config = SaadcConfig { 
        resolution: Resolution::_12BIT,
        oversample: Oversample::BYPASS,
        ..SaadcConfig::default()
    };
    let saadc = Saadc::new(saadc, saadc_config);
    saadc
}

pub(crate) fn read(mut cx: __rtic_internal_read_temperature_Context) {
    let saadc = cx.local.saadc;
    let saadc_pin = cx.local.saadc_pin;

    // NOTE: This is a blocking call, measured 49 us or 3183 cycles
    let adc_value = saadc.read_channel(saadc_pin).unwrap();
    let temp = calculate_temperature(adc_value);

    // Only update temperature if it is within the limits
    if temp >= LOWER_LIMIT && temp <= UPPER_LIMIT {
        cx.shared.temperature.lock(|temperature| {
            *temperature = temp;
        });
    }
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