use {
    crate::{app::*, state_machine::*},
    nrf52833_hal as hal, 
    nrf52833_hal::{
        lpcomp::LpCompInputPin,
        pac::LPCOMP,
    },
    panic_rtt_target as _, 
};

#[cfg(feature = "52833-debug")]
use {
    core::fmt::Write,
    rtic::Mutex,
};

pub(crate) fn init<T: LpCompInputPin>(device: LPCOMP, vdetect_pin: T) -> hal::lpcomp::LpComp {
    let comp = hal::lpcomp::LpComp::new(device, &vdetect_pin);
    comp.vref(hal::lpcomp::VRef::_4_8Vdd);
    comp.enable_interrupt(hal::lpcomp::Transition::Cross);
    comp.enable();

    comp
}

#[allow(unused_mut)]
pub(crate) fn comp_lcomp(mut cx: comp_lcomp::Context) {
    let comp = cx.local.comp;

    if comp.is_up() {
        #[cfg(feature = "52833-debug")]
        cx.shared.rtt_hw.lock(|rtt_hw| {
            writeln!(rtt_hw, "VBUS Connected").ok();
        });
        state_machine::spawn(Event::VBUSConnected).ok();
    }

    if comp.is_down() {
        #[cfg(feature = "52833-debug")]
        cx.shared.rtt_hw.lock(|rtt_hw| {
            writeln!(rtt_hw, "VBUS Disconnected").ok();
        });
        state_machine::spawn(Event::VBUSDisconnected).ok();
    }

    comp.reset_events();
}