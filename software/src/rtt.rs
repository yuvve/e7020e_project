use rtt_target::{rtt_init, UpChannel, ChannelMode};

pub(crate) fn init() -> (UpChannel, UpChannel, UpChannel, UpChannel, UpChannel) {
    let channels = rtt_init!(
        up: {
            0 : {
                size: 128,
                mode: ChannelMode::BlockIfFull,
                name: "General",
            }
            1: {
                size: 128,
                mode: ChannelMode::BlockIfFull,
                name:"Display",
            }
            2 : {
                size: 128,
                mode: ChannelMode::BlockIfFull,
                name: "HW Interrupts",
            }
            3 : {
                size: 128,
                mode: ChannelMode::BlockIfFull,
                name: "State Machine",
            }
            4 : {
                size: 128,
                mode: ChannelMode::BlockIfFull,
                name: "Serial Com",
            }
        }
    );
    (channels.up.0, channels.up.1, channels.up.2, channels.up.3, channels.up.4)
}