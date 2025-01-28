# Sea Breeze Clock

The Sea Breeze Clock provides ultimate comfort for deep sleepers. Featuring a built-in humidifier, temperature sensor displayed on screen, and a gentle wake-up vibration function, it is designed to help you start your day feeling refreshed as if you have woken up on a private island.

## Members

- Yuval Levy (yuvlev-1@student.ltu.se)
- Calle Rautio (calrau-1@student.ltu.se)
- Simon Larsson (simlar-0@student.ltu.se)

## Hardware Features

- Display
- Humidifier for sea effect
- RTC Battery
- Speaker
- User interface (encoder / button)
- Programming interface
- Temperature measurement
- Fan for (hamster sized) sea breeze effect
- USB MINI connection
- Haptic actuator for SUPER GENTLE WAKE
- Hard(ish) protective case for the board

## Functionality and SW features

Besides mandatory functionality the Final Countdown features:

- Set Clock with long push and alarm time with short push. Change of volume with rotary button on default screen.
- CLI can control temperature sensor, speaker volume, intensity of buzzer, and fan speed control from CLI, and everything else.
- Maybe additional features.

## Individual grading goals and contributions

- Yuval Levy 3) Contribute to the mandatory goals 4) ... 5) ...

- Calle Rautio 3) Contribute to the mandatory goals 4) Help with additional features 5) TBD

- Simon Larsson 3) Contribute to the mandatory goals 4) ... 5) ...

## HW References

- [I2C Display](https://se.rs-online.com/web/p/oled-displays/2543581)
- [Humidifier for sea effect]() **DATASHEET MISSING!**
- [Battery backup for RTC and alarm time (CR2032)](https://se.rs-online.com/web/p/battery-holders/2378382?gb=s)
- [Miniature speaker](https://se.rs-online.com/web/p/miniature-speakers/2596233)
- [Amplifier for the speaker]() **DATASHEET MISSING!**
- [Rotary encoder, preferably with built-in putton]() **DATASHEET MISSING!** if no push button included, use extra button
- [Push button for resetting the program]() (**included**, alternatives:  TE Connectivity 1825910-7,  Alps Alpine STTSKHHBS)
- [Haptic Actuator](https://se.farnell.com/pui-audio/hd-emc1203-lw20-r/dc-motor-3vdc-26ohm-12000rpm/dp/4411154)
- [Fan for wind effect](https://se.rs-online.com/web/p/axial-fans/2887621?gb=s) (hamster sized)
- LED (**included**)
- USB Mini (**included**)
- [Analog temperature sensor](https://www.digikey.se/sv/products/detail/epcos-tdk-electronics/B57891M0103K000/3500546) (**included**)
- Serial programming header (**included**)
- Serial communication header to host, over dev-kit VCP (**included**)
- Case made either of wood or 3D-printed
- Buzzer?

## SW References

- Crates used in the project...
