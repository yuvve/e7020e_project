use {
    hal::pac::{NVMC, UICR},
    nrf52833_hal as hal,
};

const RESET_PIN: u8 = 18;
const RESET_PORT: bool = false; // Port 0

pub(crate) fn init(uicr: UICR, nvmc: NVMC) {
    // Check if UICR is set correctly
    let check_uicr_set = uicr.nfcpins.read().protect().is_disabled()
        | uicr.pselreset[0].read().connect().is_connected()
        | uicr.pselreset[1].read().connect().is_connected();

    if !check_uicr_set {
        nvmc.config.write(|w| w.wen().wen());
        while nvmc.ready.read().ready().is_busy() {}

        // Set NFC pins to normal GPIO
        uicr.nfcpins.write(|w| w.protect().disabled());
        while nvmc.ready.read().ready().is_busy() {}

        // Set nReset pin
        for i in 0..2 {
            uicr.pselreset[i].write(|w| {
                w.pin().variant(RESET_PIN);
                w.port().variant(RESET_PORT);
                w.connect().connected();
                w
            });
            while !nvmc.ready.read().ready().is_ready() {}
        }
        nvmc.config.write(|w| w.wen().ren());
        while nvmc.ready.read().ready().is_busy() {}

        // Changes to UICR require a reset to take effect
        cortex_m::peripheral::SCB::sys_reset();
    }
}
