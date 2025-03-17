
use {
    crate::{app::*, rtc::*,},
    core::fmt::Write, 
    embedded_graphics::{
        mono_font::{ascii::FONT_10X20, MonoTextStyle}, 
        pixelcolor::BinaryColor, 
        prelude::*, 
        text::Text
    }, 
    hal::{
        pac::TWIM0, 
        twim::{Pins, Twim}
    }, 
    heapless::String, 
    nrf52833_hal::{self as hal}, 
    panic_rtt_target as _, 
    rtic::Mutex, 
    rtt_target::rprintln, 
    ssd1306::{mode::BufferedGraphicsMode, prelude::*, I2CDisplayInterface, Ssd1306},
};

const DISPLAY_STYLE: MonoTextStyle<BinaryColor> = MonoTextStyle::new(&FONT_10X20, BinaryColor::On,);

pub type Display = Ssd1306<I2CInterface<Twim<TWIM0>>, DisplaySize128x64, BufferedGraphicsMode<DisplaySize128x64>>;

pub(crate) enum Section {
    Hour,
    Minute,
    Display
}

pub(crate) fn init(twim0: TWIM0, twim_pins: Pins) -> Display{
        let i2c = Twim::new(twim0, twim_pins, hal::twim::Frequency::K100);
        let interface = I2CDisplayInterface::new(i2c);
        let mut disp: Ssd1306<_, DisplaySize128x64, BufferedGraphicsMode<DisplaySize128x64>> =
            Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
                .into_buffered_graphics_mode();

        //disp.init().unwrap();
        //disp.clear(BinaryColor::Off).unwrap();
        //disp.flush().unwrap();
        disp
}

// For debugging purposes
pub(crate) fn update_display_rtt(mut cx: update_display::Context, ticks: u32, section: Section, blink: bool) {
    let temperature = cx.shared.temperature.lock(|temperature| *temperature);
    let (hour, minute) = ticks_to_time(ticks as u32);

    if blink && !*cx.local.on {
        match section {
            Section::Hour => {
                rprintln!("Time:   :{:02}, Temperature: {:.1}", minute, temperature);
            }
            Section::Minute => {
                rprintln!("Time: {:02}:  , Temperature: {:.1}", hour, temperature);
            }
            Section::Display => {
                rprintln!("(super gentle alarm)");
            }
        }
    } else {
        rprintln!("Time: {:02}:{:02}, Temperature: {:.1}",hour,  minute, temperature);
    }

    if blink {
        *cx.local.on = !*cx.local.on;
    }
}

pub(crate) fn update_display(mut cx: update_display::Context, ticks: u32, section: Section, blink: bool) {
    let temperature = cx.shared.temperature.lock(|temperature| *temperature);
    let temperature_str = format_temperature(temperature);
    
    let (hour, minute) = ticks_to_time(ticks as u32);
    let (hour_str, minute_str) = format_time(hour, minute);

    cx.shared.display.lock(|disp| {
        disp.clear(BinaryColor::Off).unwrap();
        if blink && !*cx.local.on {
            match section {
                Section::Hour => {
                    draw_colon(disp, &DISPLAY_STYLE);
                    draw_minute(disp, &minute_str, &DISPLAY_STYLE);
                    draw_temperature(disp, &temperature_str, &DISPLAY_STYLE);
                }
                Section::Minute => {
                    draw_hour(disp, &hour_str, &DISPLAY_STYLE);
                    draw_colon(disp, &DISPLAY_STYLE);
                    draw_temperature(disp, &temperature_str, &DISPLAY_STYLE);
                }
                Section::Display => {
                }
            }
        } else {
            draw_hour(disp, &hour_str, &DISPLAY_STYLE);
            draw_colon(disp, &DISPLAY_STYLE);
            draw_minute(disp, &minute_str, &DISPLAY_STYLE);
            draw_temperature(disp, &temperature_str, &DISPLAY_STYLE);
        }
    });

    if blink {
        *cx.local.on = !*cx.local.on;
    }
}

pub(crate) fn clear(mut cx: clear_display::Context) {
    cx.shared.display.lock(|display| {
        display.clear(BinaryColor::Off).unwrap();
        display.flush().unwrap();
    });
}

fn format_time(hour: u8, minute: u8) -> (String<10>, String<10>) {
    let mut hour_str: String<10> = String::new();
    let mut minute_str: String<10> = String::new();
    core::write!(&mut hour_str, "{:02}", hour).unwrap();
    core::write!(&mut minute_str, "{:02}", minute).unwrap();
    (hour_str, minute_str)
}

fn format_temperature(temperature: f32) -> String<10> {
    let mut temp_str: String<10> = String::new();
    core::write!(&mut temp_str, "{:.2} C", temperature).unwrap();
    temp_str
}

fn draw_hour(disp: &mut Ssd1306<I2CInterface<Twim<TWIM0>>, DisplaySize128x64, BufferedGraphicsMode<DisplaySize128x64>>, time_str: &str, style: &MonoTextStyle<BinaryColor>) {
    Text::new(time_str, Point::new(24, 20), *style)
        .draw(disp)
        .unwrap();
}

fn draw_colon(disp: &mut Ssd1306<I2CInterface<Twim<TWIM0>>, DisplaySize128x64, BufferedGraphicsMode<DisplaySize128x64>>, style: &MonoTextStyle<BinaryColor>) {
    Text::new(":", Point::new(50, 20), *style)
        .draw(disp)
        .unwrap();
}

fn draw_minute(disp: &mut Ssd1306<I2CInterface<Twim<TWIM0>>, DisplaySize128x64, BufferedGraphicsMode<DisplaySize128x64>>, time_str: &str, style: &MonoTextStyle<BinaryColor>) {
    Text::new(time_str, Point::new(24, 20), *style)
        .draw(disp)
        .unwrap();
}

fn draw_temperature(disp: &mut Ssd1306<I2CInterface<Twim<TWIM0>>, DisplaySize128x64, BufferedGraphicsMode<DisplaySize128x64>>, temp_str: &str, style: &MonoTextStyle<BinaryColor>) {
    Text::new(temp_str, Point::new(50, 50), *style)
        .draw(disp)
        .unwrap();
}