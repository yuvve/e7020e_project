# Sea Breeze Clock

The Sea Breeze Clock provides ultimate comfort for deep sleepers. Featuring a built-in humidifier, temperature sensor displayed on screen, and a gentle wake-up vibration function, it is designed to help you start your day feeling refreshed as if you have woken up on a private island.

## Members

- Yuval Levy (yuvlev-1@student.ltu.se)
- Calle Rautio (calrau-1@student.ltu.se)
- Simon Larsson (simlar-0@student.ltu.se)

## Hardware Features

- Display
- Humidifier for sea effect
- Backup battery for RTC and alarm time
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

### Yuval Levy
#### 3 
- Contribute to the mandatory goals
#### 4
- Backup battery (HW & SW)
- Haptic actuator (HW & SW)
#### 5 
- Speaker (HW & SW)
- Power optimization (SW)
- Humidifier HW

### Calle Rautio
#### 3 
- Contribute to the mandatory goals
#### 4
- Humidifier SW
- Fan HW
- Fan SW
- HW component optimization (fewer MOSFETS)
#### 5 
- Speaker (HW & SW)

### Simon Larsson
#### 3 
- Contribute to the mandatory goals
#### 4
- Backup battery (HW & SW)
- Temperature on display SW
- Rotary encoder HW
#### 5 
- Speaker (HW & SW)
- Rotary encoder SW
- Power optimization (SW)

## HW References

- [MCU (QDAA QFN40)](https://docs.nordicsemi.com/bundle/ps_nrf52833/page/keyfeatures_html5.html) (**included**)
- [I2C Display](https://se.rs-online.com/web/p/oled-displays/2543581)
- [Humidifier for sea effect]() **DATASHEET MISSING!** [application](https://media.discordapp.net/attachments/1330909785532403752/1331984447591157781/temp.jpg?ex=679c2c6f&is=679adaef&hm=88c490139144ee49c3b781ae197cfe645891c9c844eaf4aa021b754f39057ddf&=&format=webp&width=810&height=403)
- [Holder for backup battery (CR2032)](https://se.rs-online.com/web/p/battery-holders/2378272?gb=s)
- [Miniature speaker](https://se.rs-online.com/web/p/miniature-speakers/2596233)
- [DAC and amplifier for the speaker](https://se.farnell.com/texas-instruments/tas5722lrsmr/amplifier-class-d-15w-vqfn-32/dp/2748912)
- [Rotary encoder, preferably with built-in button](https://se.rs-online.com/web/p/mechanical-rotary-encoders/7377773)
- [Push button for resetting the program]() (**included**, alternatives:  TE Connectivity 1825910-7,  Alps Alpine STTSKHHBS)
- [Haptic Actuator](https://se.farnell.com/pui-audio/hd-emc1203-lw20-r/dc-motor-3vdc-26ohm-12000rpm/dp/4411154)
- [Fan for wind effect](https://se.rs-online.com/web/p/axial-fans/2887621?gb=s) (hamster sized)
- LED (**included**)
- USB Mini (**included**)
- [Analog temperature sensor](https://www.digikey.se/sv/products/detail/epcos-tdk-electronics/B57891M0103K000/3500546) (**included**)
- Serial programming header (**included**)
- Serial communication header to host, over dev-kit VCP (**included**)
- Case made either of wood or 3D-printed

## SW References

- Crates used in the project...
