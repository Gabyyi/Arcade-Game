# Arcade Game

## Instalation

Before installing this, please make sure that you have installed:
  - Rust
  - probe-rs

To successfully run this rust application, you have to follow these steps:

  1. Clone the repository `git clone https://github.com/UPB-PMRust-Students/project-Gabyyi.git`

  2. Change directory to the project `cd project-Gabyyi`
      - Navigate to `cd project`
        - this is the actual poject
        - inside `src/bin` folder there are multiple versions of the code each one being an improvement variant of the previous one
          - `project.rs` - basic graphic interface and leds
          - `test.rs` - basic slots game
          - `buzz.rs` - added buzzer
          - `rfid.rs` - integration of rfid module
          - `eeprom.rs` - integratio of memory module
          - `image.rs` full game
      - Navigate to `cd project_second_display`
        - this is a separate project for the secondary display
          - the code for the secondary display is written in another rust project because of a conflict between `embedded-graphics` versions used by each display
          - the secondary display is connected to another Raspberry Pi Pico 2W because of the lack of free pins on the main Pico   
  3. Build the project `cargo build`
  
  4. Run the command to flash on the Pico `cargo run --bin image.rs`

## Description

The project uses two Raspberry Pi Pico 2W as the control units, along with two displays — a main display showing the slot machine game and a secondary display showing the winning combinations. The balance is simulated using an RFID card reader and a memory module. For an even better simulation, LEDs and a passive buzzer are used for audio-visual effects.

## Hardware

| Device | Usage | Price |
|--------|--------|-------|
| [Raspberry Pi Pico 2W](https://www.raspberrypi.com/documentation/microcontrollers/pico-series.html) | The microcontroller x2 | [39.66 RON](https://www.optimusdigital.ro/ro/placi-raspberry-pi/13327-raspberry-pi-pico-2-w.html) |
| [ILI9341](https://cdn-shop.adafruit.com/datasheets/ILI9341.pdf) | Main display | [69.99 RON](https://www.optimusdigital.ro/ro/optoelectronice-lcd-uri/3550-modul-lcd-de-28-cu-spi-i-controller-ili9341-240x320-px.html) |
| [ST7735](https://www.hpinfotech.ro/ST7735S.pdf) | Secondary display | [27.99 RON](https://www.optimusdigital.ro/ro/optoelectronice-lcd-uri/870-modul-lcd-144.html) |
| [RFID MFRC522](https://www.nxp.com/docs/en/data-sheet/MFRC522.pdf) | Card Reader | [9.99 RON](https://www.optimusdigital.ro/ro/wireless-rfid/67-modul-cititor-rfid-mfrc522.html) |
| [AT24C256](https://ww1.microchip.com/downloads/en/DeviceDoc/doc0670.pdf) | Stores the balance | [8.99 RON](https://www.optimusdigital.ro/ro/memorii/632-modul-eeprom-at24c256.html) |
| [Passive Buzzer](https://www.handsontec.com/dataspecs/module/passive%20buzzer.pdf) | Audio Effects | [1.69 RON](https://www.optimusdigital.ro/ro/componente-electronice/12598-modul-buzzer-pasiv.html) |
| [Red Cap Button](https://baltacom.com/upload/uf/546/5462ae185a73b708af4e9df5946d9a3e.pdf) | x4 | [1.99 RON](https://www.optimusdigital.ro/ro/butoane-i-comutatoare/1114-buton-cu-capac-rotund-rou.html) |
| [LEDs](https://www.farnell.com/datasheets/1498852.pdf) | x8 (from kit)| [26.99 RON](https://www.optimusdigital.ro/ro/kituri-optimus-digital/9517-set-de-led-uri-asortate-de-5-mm-si-3-mm-310-buc-cu-rezistoare-bonus.html) |
| [220Ω Resistors](https://www.optimusdigital.ro/ro/kituri-optimus-digital/9517-set-de-led-uri-asortate-de-5-mm-si-3-mm-310-buc-cu-rezistoare-bonus.html) | x8 (from kit)| [26.99 RON](https://www.optimusdigital.ro/ro/kituri-optimus-digital/9517-set-de-led-uri-asortate-de-5-mm-si-3-mm-310-buc-cu-rezistoare-bonus.html) |

## Links

1. [Personal Repo](https://github.com/Gabyyi/Arcade-Game)
