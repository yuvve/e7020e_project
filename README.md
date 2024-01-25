# e7020e Project Name: <e.g., Final Countdown>

<short selling description, e.g., Final Countdown provides the ultimate waker experience combining latest innovative software and hardware technology into a climate neutral product free of cultural appropriation.>

## Members

- Anders Petterson (andpe0@student.ltu.se)
- Anna Broman (anabro1@student.ltu.se)
- Jacob Gonzales (jago3@student.ltu.se)

## Hardware Features

- <e.g., oled screen>
- <e.g., (4) buttons for navigation>
- <e.g., battery backup of RTC>
- <e.g., power over usb>

and mandatory components for:

- Analog sensing <e.g., photo resistor>
- Serial communication interface to host, <e.g. over dev-kit VCP>
- LED
- Buzzer

## Functionality and SW features

Besides mandatory functionality the Final Countdown features:

<e.g.>

- Automatic RTC update by UTC network time and Time Zone alignment
- Alarm off functionality requiring the user to guess a random number between 0 and 20 using 3 attempts at most. Feedback given as "too low", "too high" or "you got it!" which turns off the alarm. Fail creates a new random number for the user to guess until correct number is guessed.
- Ability to read and set a new alarm from device using the (4) navigation buttons.

## Individual grading goals and contributions

<e.g.>

- Anders Petterson 3) Contribute to the mandatory goals 4) Prototyping the guessing game first on host, later porting to target including OLED SW. 5) TBD

- Anna Broman 3) Contribute to the mandatory goals 4) Prototyping UTC/Localization on host, later porting to target including RTC read/write functionality. 5) TBD

- Jacob Gonzales 3) Contribute to the mandatory goals 4) Battery backup design, power measurements, and low-power optimization 5) TBD

## HW References

<e.g.>

- [oled display](https://en.odroid.se/products/0-96-tum-oled-spi-i2c-granssnitt-vinklad-horisontell-pinheader?pr_prod_strat=e5_desc&pr_rec_id=b23563853&pr_rec_pid=6585308020814&pr_ref_pid=6585303924814&pr_seq=uniform/)

## SW References

<e.g.>

- [Chrono] (https://crates.io/crates/chrono)
- [ssd1306] (https://docs.rs/ssd1306/latest/ssd1306/)
- [Embedded Graphics] (https://docs.rs/embedded-graphics/latest/embedded_graphics/)
