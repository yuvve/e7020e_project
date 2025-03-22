use {
    crate::{app::*, rtc, display}, core::sync::atomic::Ordering,panic_rtt_target as _
};

#[cfg(feature = "52833-debug")]
use {
    core::fmt::Write,
    rtic::Mutex,
};

pub const DATA_OUT_BUFFER_SIZE: usize = 64;
pub const DATA_IN_BUFFER_SIZE: usize = 64;

pub(crate) enum CliCommand {
    SetTime(u8, u8),
    SetAlarm(u8, u8),
    GetTime,
    GetAlarm,
}

#[allow(unused_mut)]
#[allow(unused_variables)]
pub(crate) fn cli_commands(mut cx: cli_commands::Context, command: CliCommand) {
    match command {
        CliCommand::SetTime(hour, minute) => {
            #[cfg(feature = "52833-debug")]
            cx.shared.rtt_serial.lock(|rtt_serial| {
                writeln!(rtt_serial, "Set time: {:02}:{:02}", hour, minute).ok();
            });
        
            let mut time = [0u8; 5];
            time_formatter(hour, minute, &mut time);
            let msg = b"Time set to ";
            let mut data = [0u8; 17];
            data[0..12].copy_from_slice(msg);
            data[12..17].copy_from_slice(&time);
            write_to_serial(&data);
            
            let ticks = rtc::time_to_ticks(hour, minute);
            set_time::spawn(ticks).ok();
            update_display::spawn(ticks, display::Section::Display, false).ok();
        }
        CliCommand::SetAlarm(hour, minute) => {
            #[cfg(feature = "52833-debug")]
            cx.shared.rtt_serial.lock(|rtt_serial| {
                writeln!(rtt_serial, "Set alarm: {:02}:{:02}", hour, minute).ok();
            });
            
            let mut time = [0u8; 5];
            time_formatter(hour, minute, &mut time);
            let msg = b"Alarm set to ";
            let mut data = [0u8; 18];
            data[0..13].copy_from_slice(msg);
            data[13..18].copy_from_slice(&time);
            write_to_serial(&data);

            let ticks = rtc::time_to_ticks(hour, minute);
            set_alarm::spawn(ticks).ok();
        }
        CliCommand::GetTime => {
            let curr_time_ticks = cx.shared.time_offset_ticks.load(Ordering::Relaxed);
            let (hour, minute) = rtc::ticks_to_time(curr_time_ticks);

            #[cfg(feature = "52833-debug")]
            cx.shared.rtt_serial.lock(|rtt_serial| {
                writeln!(rtt_serial, "Get time: {:02}:{:02}", hour, minute).ok();
            });

            let mut time = [0u8; 5];
            time_formatter(hour, minute, &mut time);
            let msg = b"Current time: ";
            let mut data = [0u8; 19];
            data[0..14].copy_from_slice(msg);
            data[14..19].copy_from_slice(&time);
            write_to_serial(&data);
        }
        CliCommand::GetAlarm => {
            let curr_alarm_ticks = cx.shared.alarm_offset_ticks.load(Ordering::Relaxed);
            let (hour, minute) = rtc::ticks_to_time(curr_alarm_ticks);

            #[cfg(feature = "52833-debug")]
            cx.shared.rtt_serial.lock(|rtt_serial| {
                writeln!(rtt_serial, "Get alarm: {:02}:{:02}", hour, minute).ok();
            });

            let mut time = [0u8; 5];
            time_formatter(hour, minute, &mut time);
            let msg = b"Current alarm: ";
            let mut data = [0u8; 20];
            data[0..15].copy_from_slice(msg);
            data[15..20].copy_from_slice(&time);
            write_to_serial(&data);
        }
    }
}

fn time_formatter(hour: u8, minute: u8, buffer: &mut [u8; 5]){
    buffer[0] = (hour / 10) + b'0';
    buffer[1] = (hour % 10) + b'0';
    buffer[2] = b':';
    buffer[3] = (minute / 10) + b'0';
    buffer[4] = (minute % 10) + b'0';
}

pub(crate) fn parse_serial_cmd(bytes: &[u8]) -> Option<CliCommand> {
    let mut split = bytes.splitn(bytes.len(), |c| *c == b' ');
    let cmd = split.next()?;
    match cmd {
        b"set" => {
            let arg = split.next()?;
            match arg {
                b"time" => {
                    let next = split.next()?;
                    let hour_minute = next.splitn(next.len(), |c| *c == b':');
                    
                    let mut split = hour_minute;
                    let next = split.next()?;
                    let str = core::str::from_utf8(next).ok()?;
                    let hour: u8 = str.parse().ok()?;

                    let next = split.next()?;
                    let str = core::str::from_utf8(next).ok()?;
                    let minute: u8 = str.parse().ok()?;

                    Some(CliCommand::SetTime(hour, minute))
                }
                b"alarm" => {
                    let next = split.next()?;
                    let hour_minute = next.splitn(next.len(), |c| *c == b':');
                    
                    let mut split = hour_minute;
                    let next = split.next()?;
                    let str = core::str::from_utf8(next).ok()?;
                    let hour: u8 = str.parse().ok()?;
                    
                    let next = split.next()?;
                    let str = core::str::from_utf8(next).ok()?;
                    let minute: u8 = str.parse().ok()?;

                    Some(CliCommand::SetAlarm(hour, minute))
                }
                _ => None,
            }
        }
        b"get" => {
            let next = split.next()?;
            match next {
                b"time" => Some(CliCommand::GetTime),
                b"alarm" => Some(CliCommand::GetAlarm),
                _ => None,
            }
        }
        _ => None,
    }
}

// Should NOT be RTIC task
// Just makes it easier to create an array of the correct size
pub(crate) fn write_to_serial(data: &[u8]) {
    //writeln!(rtt_channel, "write_to_serial: {:?}", core::str::from_utf8(data)).unwrap();

    let mut len: usize = data.len();
    let size_with_newline = DATA_OUT_BUFFER_SIZE -1;
    if len > size_with_newline {
        //writeln!(rtt_channel, "Data too large, truncating").unwrap();

        len = size_with_newline;
    }
    let mut data_out = [0u8; DATA_OUT_BUFFER_SIZE];
    data_out[0..len].copy_from_slice(data);
    data_out[len] = 13;
    data_out::spawn(data_out, len+1).unwrap();
}

#[allow(unused_mut)]
pub(crate) fn data_out(mut cx: data_out::Context, data: [u8; DATA_OUT_BUFFER_SIZE], len: usize) {
    let serial = cx.shared.serial;
    let usb_dev = cx.shared.usb_dev;

    match serial.write(&data[0..len]) {
        Ok(_) => {
            usb_dev.poll(&mut [serial]);
        }
        Err(_) => {
            #[cfg(feature = "52833-debug")]
            cx.shared.rtt_serial.lock(|rtt_serial| {
                writeln!(rtt_serial, "Error writing data").ok();
            });
        }
    }
}

#[allow(unused_mut)]
pub(crate) fn data_in(mut cx: data_in::Context, data: u8) {
    let len = cx.local.len;
    let data_arr = cx.local.data_arr;

    match data {
        13 => {
            let slice = &data_arr[0..*len];
            #[cfg(feature = "52833-debug")]
            cx.shared.rtt_serial.lock(|rtt_serial| {
                writeln!(rtt_serial, "Received: {:?}", core::str::from_utf8(slice)).ok();
            });

            if let Some(command) = parse_serial_cmd(slice) {
                cli_commands::spawn(command).ok();
            } else {
                #[cfg(feature = "52833-debug")]
                cx.shared.rtt_serial.lock(|rtt_serial| {
                    writeln!(rtt_serial, "Invalid command").ok();
                });

                write_to_serial(b"Invalid command or argument");
            }
            *len = 0;
        }
        _ => {
            data_arr[*len] = data;
            if *len < data_arr.len() - 1 {
                *len += 1;
            } else {
                #[cfg(feature = "52833-debug")]
                cx.shared.rtt_serial.lock(|rtt_serial| {
                    writeln!(rtt_serial, "Buffer full, discarding data").ok();
                });
                *len = 0;
            }
        }
    }
}

#[allow(unused_mut)]
pub(crate) fn usb_fs(mut cx: usb_fs::Context) {
    #[cfg(feature = "52833-debug")]
    cx.shared.rtt_hw.lock(|rtt_hw| {
        writeln!(rtt_hw, "USBD interrupt").ok();
    });
    let usb_dev = cx.shared.usb_dev;
    let serial = cx.shared.serial;
    
    let mut buf = [0u8; 64];
    usb_dev.poll(&mut [serial]);

    match serial.read(&mut buf) {
        Ok(count) if count > 0 => {
            for i in 0..count {
                data_in::spawn(buf[i]).unwrap();
            }
        }
        _ => {}
    }
}