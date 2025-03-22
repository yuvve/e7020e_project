use {
    crate::{app::*, rtc::*},
    embedded_graphics::{
        mono_font::MonoTextStyle,
        pixelcolor::BinaryColor,
        prelude::*,
        text::Text,
    },
    hal::{
        pac::TWIM0,
        twim::{Pins, Twim},
    },
    heapless::String,
    nrf52833_hal as hal,
    panic_rtt_target as _,
    profont::*,
    rtic::Mutex,
    ssd1306::{mode::BufferedGraphicsMode, prelude::*, I2CDisplayInterface, Ssd1306},
};

#[cfg(feature = "52833-debug")]
use core::fmt::Write;

const TIME_DISPLAY_STYLE: MonoTextStyle<BinaryColor> = MonoTextStyle::new(&PROFONT_24_POINT, BinaryColor::On);
const TEMP_DISPLAY_STYLE: MonoTextStyle<BinaryColor> = MonoTextStyle::new(&PROFONT_14_POINT, BinaryColor::On);

const FONT_SIZE: Point = Point::new(16, 29);

const TIME_POSITION: Point = Point::new(24, 20);
const HOUR_POSITION: Point = Point::new(TIME_POSITION.x, TIME_POSITION.y);
const COLON_POSITION: Point = Point::new(HOUR_POSITION.x + (FONT_SIZE.x * 2), TIME_POSITION.y);
const MINUTE_POSITION: Point = Point::new(COLON_POSITION.x + FONT_SIZE.x, TIME_POSITION.y);
const TEMPERATURE_POSITION: Point = Point::new(35, 50);
const ALARM_POSITION: Point = Point::new(MINUTE_POSITION.x + (FONT_SIZE.x * 2), TIME_POSITION.y);
const ALARM_STRING: &str = "(«";

pub type Display =
    Ssd1306<I2CInterface<Twim<TWIM0>>, DisplaySize128x64, BufferedGraphicsMode<DisplaySize128x64>>;

#[derive(PartialEq)]
pub(crate) enum Section {
    Hour,
    Minute,
    Display,
    AlarmIcon,
}

pub(crate) fn init(twim0: TWIM0, twim_pins: Pins) -> Display {
    let i2c = Twim::new(twim0, twim_pins, hal::twim::Frequency::K100);
    let interface = I2CDisplayInterface::new(i2c);
    let mut disp: Ssd1306<_, DisplaySize128x64, BufferedGraphicsMode<DisplaySize128x64>> =
        Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();

    disp.init().unwrap();
    disp.clear(BinaryColor::Off).unwrap();
    disp.flush().unwrap();

    disp
}

#[allow(unused_variables)]
pub(crate) fn update_display_rtt(
    mut cx: update_display::Context,
    ticks: u32,
    section: Section,
    blink: bool,
) {
    let temperature = cx.shared.temperature.lock(|temperature| *temperature);
    let (hour, minute) = ticks_to_time(ticks as u32);

    #[cfg(feature = "52833-debug")]
    if blink && !*cx.local.on {
        match section {
            Section::Hour => {
                writeln!(cx.local.rtt_display, "   :{:02}     {:.1} C", minute, temperature).ok();
            }
            Section::Minute => {
                writeln!(cx.local.rtt_display, " {:02}:       {:.1} C", hour, temperature).ok();
            }
            Section::Display => {
                writeln!(cx.local.rtt_display, "(super gentle alarm)").ok();
            }
            Section::AlarmIcon => {
                writeln!(cx.local.rtt_display, " {:02}:{:02}     {:.1} C  {}", hour, minute, temperature, ALARM_STRING).ok();
            }
        }
    } else {
        writeln!(cx.local.rtt_display, 
            " {:02}:{:02}     {:.1} C",
            hour,
            minute,
            temperature
        ).ok();
    }

    if blink {
        *cx.local.on = !*cx.local.on;
    }
}

pub(crate) fn update_display(
    mut cx: update_display::Context,
    ticks: u32,
    section: Section,
    blink: bool,
) {
    #[cfg(feature = "52833-debug")]
    writeln!(cx.local.rtt_display, "Updating display...").ok();
    let temperature = cx.shared.temperature.lock(|temperature| *temperature);
    let temperature_str = format_temperature(temperature);

    let (hour, minute) = ticks_to_time(ticks as u32);
    let (hour_str, minute_str) = format_time(hour, minute);

    cx.shared.display.lock(|disp| {
        disp.clear(BinaryColor::Off).unwrap();
        if blink && !*cx.local.on {
            #[cfg(feature = "52833-debug")]
            writeln!(cx.local.rtt_display, "Blinking...").ok();
            match section {
                Section::Hour => {
                    draw_colon(disp);
                    draw_minute(disp, &minute_str);
                    draw_temperature(disp, &temperature_str);
                }
                Section::Minute => {
                    draw_hour(disp, &hour_str);
                    draw_colon(disp);
                    draw_temperature(disp, &temperature_str);
                }
                Section::Display => {}
                Section::AlarmIcon => {            
                    draw_hour(disp, &hour_str);
                    draw_colon(disp);
                    draw_minute(disp, &minute_str);
                    draw_temperature(disp, &temperature_str);
                    draw_alarm_icon(disp);
                }
            }
        } else {
            draw_hour(disp, &hour_str);
            draw_colon(disp);
            draw_minute(disp, &minute_str);
            draw_temperature(disp, &temperature_str);
            if Section::AlarmIcon == section {
                draw_alarm_icon(disp);
            }
        }
        disp.flush().unwrap();
    });

    if blink {
        *cx.local.on = !*cx.local.on;
    }
}

#[allow(unused_variables)]
#[allow(unused_mut)]
fn format_time(hour: u8, minute: u8) -> (String<10>, String<10>) {
    let mut hour_str: String<10> = String::new();
    let mut minute_str: String<10> = String::new();
    #[cfg(feature = "52833-debug")]
    core::write!(&mut hour_str, "{:02}", hour).unwrap();
    #[cfg(feature = "52833-debug")]
    core::write!(&mut minute_str, "{:02}", minute).unwrap();
    (hour_str, minute_str)
}

#[allow(unused_variables)]
#[allow(unused_mut)]
fn format_temperature(temperature: f32) -> String<10> {
    let mut temp_str: String<10> = String::new();
    #[cfg(feature = "52833-debug")]
    core::write!(&mut temp_str, "{:.1}°C", temperature).unwrap();
    temp_str
}

fn draw_hour(
    disp: &mut Ssd1306<
        I2CInterface<Twim<TWIM0>>,
        DisplaySize128x64,
        BufferedGraphicsMode<DisplaySize128x64>,
    >,
    time_str: &str,
) {
    Text::new(time_str, HOUR_POSITION, TIME_DISPLAY_STYLE)
        .draw(disp)
        .unwrap();
}

fn draw_colon(
    disp: &mut Ssd1306<
        I2CInterface<Twim<TWIM0>>,
        DisplaySize128x64,
        BufferedGraphicsMode<DisplaySize128x64>,
    >
) {
    Text::new(":", COLON_POSITION, TIME_DISPLAY_STYLE)
        .draw(disp)
        .unwrap();
}

fn draw_minute(
    disp: &mut Ssd1306<
        I2CInterface<Twim<TWIM0>>,
        DisplaySize128x64,
        BufferedGraphicsMode<DisplaySize128x64>,
    >,
    time_str: &str,
) {
    Text::new(time_str, MINUTE_POSITION, TIME_DISPLAY_STYLE)
        .draw(disp)
        .unwrap();
}

fn draw_temperature(
    disp: &mut Ssd1306<
        I2CInterface<Twim<TWIM0>>,
        DisplaySize128x64,
        BufferedGraphicsMode<DisplaySize128x64>,
    >,
    temp_str: &str,
) {
    Text::new(temp_str, TEMPERATURE_POSITION, TEMP_DISPLAY_STYLE)
        .draw(disp)
        .unwrap();
}

fn draw_alarm_icon(
    disp: &mut Ssd1306<
        I2CInterface<Twim<TWIM0>>,
        DisplaySize128x64,
        BufferedGraphicsMode<DisplaySize128x64>,
    >,
) {
    Text::new(ALARM_STRING, ALARM_POSITION, TIME_DISPLAY_STYLE)
        .draw(disp)
        .unwrap();
}

pub(crate) fn disable_display(mut cx: disable_display::Context) {
    cx.shared.display.lock(|disp| {
        disp.set_display_on(false).ok();
    });
}

pub(crate) fn enable_display(mut cx: enable_display::Context) {
    cx.shared.display.lock(|disp| {
        disp.set_display_on(true).ok();
    });
}