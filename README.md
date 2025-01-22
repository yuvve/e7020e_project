# e7020e Project Name: sea breeze clock

<short selling description, e.g., Final Countdown provides the ultimate waker experience combining latest innovative software and hardware technology into a climate neutral product free of cultural appropriation.>

## Members

- Yuval Levy (andpe0@student.ltu.se)
- Calle Rautio (calrau-1@student.ltu.se)
- Simon Larsson (jago3@student.ltu.se)

## Hardware Features

- Display given by Lab
- Humidifier
- <battery backup for RTC, and alarm time>
- power over usb (micro or something else)
- Tempature and humidity sensor
- inbedded Speaker, should be able to play music
- Rotary encoder, button
- Haptic Acuator
- Added Fan

and mandatory components for:

- Analog sensing <Tempature, humidity sensor>
- Serial communication interface to host, over dev-kit VCP
- LED
- Buzzer

## Functionality and SW features

Besides mandatory functionality the Final Countdown features:

- Set Clock and Alarm with rotary button, alternatively added button on device
- CLI being able to control temprature screen, humudity, speaker volume, intensity of buzzer, and fan speed control from CLI

## Individual grading goals and contributions

<e.g.>

- Anders Petterson 3) Contribute to the mandatory goals 4) Prototyping the guessing game first on host, later porting to target including OLED SW. 5) TBD

- Calle Rautio 3) Contribute to the mandatory goals 4)  5) TBD

- Jacob Gonzales 3) Contribute to the mandatory goals 4) Battery backup design, power measurements, and low-power optimization 5) TBD

## HW References

- [oled display](https://en.odroid.se/products/0-96-tum-oled-spi-i2c-granssnitt-vinklad-horisontell-pinheader?pr_prod_strat=e5_desc&pr_rec_id=b23563853&pr_rec_pid=6585308020814&pr_ref_pid=6585303924814&pr_seq=uniform/)

## SW References

- [Chrono] (https://crates.io/crates/chrono)
- [ssd1306] (https://docs.rs/ssd1306/latest/ssd1306/)
- [Embedded Graphics] (https://docs.rs/embedded-graphics/latest/embedded_graphics/)
