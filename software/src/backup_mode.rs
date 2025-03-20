use {
    crate::{app::*, state_machine::*},
    rtic::Mutex,
    core::fmt::Write,
    nrf52833_hal as hal, 
    nrf52833_hal::{
        lpcomp::LpCompInputPin,
        pac::LPCOMP,
    },
    panic_rtt_target as _, 
};

pub(crate) fn init<T: LpCompInputPin>(device: LPCOMP, vdetect_pin: T) -> hal::lpcomp::LpComp {
    let comp = hal::lpcomp::LpComp::new(device, &vdetect_pin);
    comp.vref(hal::lpcomp::VRef::_4_8Vdd);
    comp.enable_interrupt(hal::lpcomp::Transition::Cross);
    comp.enable();

    comp
}

pub(crate) fn comp_lcomp(mut cx: comp_lcomp::Context) {
    let comp = cx.local.comp;

    if comp.is_up() {
        cx.shared.rtt_hw.lock(|rtt_hw| {
            writeln!(rtt_hw, "VBUS Connected").ok();
        });
        state_machine::spawn(Event::VBUSConnected).ok();
    }

    if comp.is_down() {
        cx.shared.rtt_hw.lock(|rtt_hw| {
            writeln!(rtt_hw, "VBUS Disconnected").ok();
        });
        state_machine::spawn(Event::VBUSDisconnected).ok();
    }

    comp.reset_events();
}