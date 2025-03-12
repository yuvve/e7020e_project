#![no_main]
#![no_std]
#![deny(unsafe_code)]
#![deny(warnings)]

use {
    core::fmt::Write,
    nrf52833_hal as hal,
    hal::{pac, twim::Twim, gpio::{p0::Parts as P0Parts, p1::Parts as P1Parts}},
    panic_rtt_target as _,
    rtt_target::{rprintln, rtt_init_print},
    systick_monotonic::*,
    embedded_graphics::{
        mono_font::{ascii::FONT_10X20, MonoTextStyle},
        prelude::*,
        text::Text,
        pixelcolor::BinaryColor,
    },
    ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306, mode::BufferedGraphicsMode},
    heapless::String,
};

/// Display update interval (ms)
const DISPLAY_UPDATE_MS: u64 = 1000;

#[rtic::app(device = pac, dispatchers = [TIMER0])]
mod app {
    use super::*;

    #[monotonic(binds = SysTick, default = true)]
    type MyMono = Systick<1000>;

    #[shared]
    struct Shared {
        display: Ssd1306<I2CInterface<Twim<pac::TWIM0>>, DisplaySize128x64, BufferedGraphicsMode<DisplaySize128x64>>,
        time: u32, // Time counter in seconds
        temp: i32, // Temperature
    }

    #[local]
    struct Local {}

    #[init]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        let mono = Systick::new(cx.core.SYST, 64_000_000);

        rtt_init_print!();
        rprintln!("Initializing OLED display...");

        let port0 = P0Parts::new(cx.device.P0);
        let port1 = P1Parts::new(cx.device.P1);

        let scl = port0.p0_11.into_floating_input().degrade();
        let sda = port1.p1_09.into_floating_input().degrade();
        let i2c = Twim::new(cx.device.TWIM0, hal::twim::Pins { scl, sda }, hal::twim::Frequency::K100);

        let interface = I2CDisplayInterface::new(i2c);
        let mut disp: Ssd1306<_, DisplaySize128x64, BufferedGraphicsMode<DisplaySize128x64>> =
            Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
                .into_buffered_graphics_mode();

        disp.init().unwrap();
        disp.clear(BinaryColor::Off).unwrap();
        disp.flush().unwrap();

        rprintln!("Initialization complete.");

        update_display::spawn_after(DISPLAY_UPDATE_MS.millis()).unwrap();

        (Shared { display: disp, time: 0, temp: 24 }, Local {}, init::Monotonics(mono))
    }

    #[task(shared = [display, time, temp])]
    fn update_display(mut cx: update_display::Context) {
        let style = &MonoTextStyle::new(&FONT_10X20, BinaryColor::On);

        let current_time = update_time(&mut cx.shared.time);

        let time_str = format_time(current_time);
        // Logic needed for temperature to update  
        let temp_str = "20C";

        cx.shared.display.lock(|disp| {
            disp.clear(BinaryColor::Off).unwrap();

            draw_clock(disp, &time_str, style);
            draw_temperature(disp, &temp_str, style);
            // Problem with adding ° because embedded_graphics for some reason only supports ascii light and not utf-8 or extended

            disp.flush().unwrap();
        });

        update_display::spawn_after(DISPLAY_UPDATE_MS.millis()).unwrap();
    }

    // Update clock, current_time needs logic to be able to be correct
    fn update_time(time: &mut impl rtic::Mutex<T = u32>) -> u32 {
        time.lock(|t| {
            *t += 1;
            *t
        })
        // Also there is no logic for the clock to actually go back to 00:00 after 23:59
    }

    fn format_time(seconds: u32) -> String<10> {
        let hours = (seconds / 3600) % 24;
        let minutes = (seconds / 60) % 60;
        let seconds = seconds % 60;
        let mut time_str: String<10> = String::new();
        core::write!(&mut time_str, "{:02}:{:02}:{:02}", hours, minutes, seconds).unwrap();
        time_str
    }

    fn draw_clock(disp: &mut Ssd1306<I2CInterface<Twim<pac::TWIM0>>, DisplaySize128x64, BufferedGraphicsMode<DisplaySize128x64>>, time_str: &str, style: &MonoTextStyle<BinaryColor>) {
        Text::new(time_str, Point::new(24, 20), *style)
            .draw(disp)
            .unwrap();
    }

    fn draw_temperature(disp: &mut Ssd1306<I2CInterface<Twim<pac::TWIM0>>, DisplaySize128x64, BufferedGraphicsMode<DisplaySize128x64>>, temp_str: &str, style: &MonoTextStyle<BinaryColor>) {
        Text::new(temp_str, Point::new(50, 50), *style)
            .draw(disp)
            .unwrap();
    }
}
