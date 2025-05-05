#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::{
    gpio::{Input, Level, Output, Pull},
    peripherals::{SPI0, SPI1},
    pwm::{Config as ConfigPwm, Pwm, SetDutyCycle},
    spi::{Blocking, Config as ConfigSpi, Spi},
};
use embassy_sync::{blocking_mutex::NoopMutex, channel, pubsub::publisher};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, channel::Channel};
use embassy_time::{Delay, Duration, Timer};
use core::cell::RefCell;
use core::fmt::Write; // Import for core formatting
use defmt::info;
use display_interface_spi::SPIInterface;
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDevice;
use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_10X20},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Circle, PrimitiveStyle, Rectangle},
    text::Text,
};
use fixed::traits::ToFixed;
use heapless::String; // Import for no_std string handling
use ili9341::{DisplaySize240x320, DisplaySize320x480, Ili9341, ModeState, Orientation};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

static SPI_BUS: StaticCell<NoopMutex<RefCell<Spi<'static, SPI0, Blocking>>>> = StaticCell::new(); // for borrowing to a task

use embassy_futures::select::select;
use embassy_sync::pubsub::{
    PubSubChannel, Publisher, Subscriber,
    WaitResult::{Lagged, Message as wrm},
};
use embassy_time::Instant;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use mfrc522::{Mfrc522};
use embassy_rp::i2c::{I2c, InterruptHandler as I2CInterruptHandler, Config as I2cConfig, Async as I2cAsync};
use embedded_hal_async::i2c::{Error, I2c as _};
use embassy_rp::peripherals::I2C1;
use embassy_rp::bind_interrupts;
use embassy_sync::signal::Signal;

bind_interrupts!(struct Irqs {
    I2C1_IRQ => I2CInterruptHandler<I2C1>;
});

static CHANNEL: PubSubChannel<ThreadModeRawMutex, State, 1000, 5, 5> = PubSubChannel::new();

#[derive(Clone, Copy, PartialEq, defmt::Format)]
enum State {
    SPIN,
    WIN,
    BET,
    ADDBALANCE,
    CASHOUT,
}

use core::sync::atomic::{AtomicI32, Ordering};

static BALANCE: AtomicI32 = AtomicI32::new(0);

static BALANCE1: AtomicI32 = AtomicI32::new(80000);
static BALANCE2: AtomicI32 = AtomicI32::new(100000);

const UID1: [u8; 4] = [80, 243, 109, 20];
const UID2: [u8; 4] = [10, 85, 52, 0];

const EEPROM_ADDR: u8 = 0x50;
const CARD_SIZE: usize = 8; // 4 bytes UID + 4 bytes counter
const EEPROM_START_ADDR: u16 = 0x0000; // start of EEPROM
const NUM_CARDS: usize = 2;
type CardEntry = ([u8; 4], u32);

async fn write_card_data(i2c: &mut I2c<'_, I2C1, I2cAsync>) -> Result<(), embedded_hal_async::i2c::ErrorKind> {
    let cards = [
        (UID1, BALANCE1.load(Ordering::SeqCst) as u32),
        (UID2, BALANCE2.load(Ordering::SeqCst) as u32),
    ];

    for (index, (uid, balance)) in cards.iter().enumerate() {
        let mem_addr = EEPROM_START_ADDR + (index as u16) * (CARD_SIZE as u16);
        let mem_addr_bytes = mem_addr.to_be_bytes();
        let mut buffer = [0u8; 2 + CARD_SIZE];

        buffer[0..2].copy_from_slice(&mem_addr_bytes);
        buffer[2..6].copy_from_slice(uid);
        buffer[6..10].copy_from_slice(&balance.to_be_bytes());

        i2c.write(EEPROM_ADDR, &buffer).await.map_err(|_| embedded_hal_async::i2c::ErrorKind::Other)?;
        Timer::after_millis(10).await; // EEPROM write delay
    }

    info!("EEPROM initialized with known cards{},{},{},{}.", UID1, BALANCE1.load(Ordering::SeqCst), UID2, BALANCE2.load(Ordering::SeqCst));

    Ok(())
}


async fn load_card_data(i2c: &mut I2c<'_, I2C1, I2cAsync>) -> Result<(), embedded_hal_async::i2c::ErrorKind> {
    let mut read_buffer = [0u8; CARD_SIZE];

    for index in 0..NUM_CARDS {
        let mem_addr = EEPROM_START_ADDR + (index as u16) * (CARD_SIZE as u16);
        let mem_addr_bytes = mem_addr.to_be_bytes();

        i2c.write_read(EEPROM_ADDR, &mem_addr_bytes, &mut read_buffer)
            .await
            .map_err(|_| embedded_hal_async::i2c::ErrorKind::Other)?;

        let uid: [u8; 4] = read_buffer[0..4].try_into().unwrap();
        let balance = u32::from_be_bytes(read_buffer[4..8].try_into().unwrap());

        if index == 0 {
            UID1.copy_from_slice(&uid);
            BALANCE1.store(balance as i32, Ordering::SeqCst);
        } else if index == 1 {
            UID2.copy_from_slice(&uid);
            BALANCE2.store(balance as i32, Ordering::SeqCst);
        }
    }

    info!("EEPROM loaded into globals{},{},{},{}.", UID1, BALANCE1.load(Ordering::SeqCst), UID2, BALANCE2.load(Ordering::SeqCst));

    Ok(())
}




#[embassy_executor::task]
async fn display_task(
    spi_bus: &'static NoopMutex<RefCell<Spi<'static, SPI0, Blocking>>>,
    mut cs: Output<'static>,
    mut dc: Output<'static>,
    mut reset: Output<'static>,
    mut increase_bet: Input<'static>,
    mut max_bet: Input<'static>,
    mut spin_button: Input<'static>,
    mut cashout_button: Input<'static>,
) {
    let spi_dev = SpiDevice::new(&spi_bus, cs);
    let iface = SPIInterface::new(spi_dev, dc);

    let mut delay = Delay;

    let mut display = Ili9341::new(
        iface,
        reset,
        &mut delay,
        Orientation::LandscapeFlipped,
        DisplaySize240x320,
    )
    .unwrap();

    display.idle_mode(ModeState::Off).unwrap();
    display.invert_mode(ModeState::On).unwrap();
    let _ = display.normal_mode_frame_rate(
        ili9341::FrameRateClockDivision::Fosc,
        ili9341::FrameRate::FrameRate100,
    );
    display.clear(Rgb565::BLACK).unwrap();

    let rect_style = PrimitiveStyle::with_fill(Rgb565::RED); //linie sus
    Rectangle::new(Point::new(0, 0), Size::new(320, 5))
        .into_styled(rect_style)
        .draw(&mut display)
        .unwrap();

    let rect_style = PrimitiveStyle::with_fill(Rgb565::RED); //linie jos
    Rectangle::new(Point::new(0, 235), Size::new(320, 5))
        .into_styled(rect_style)
        .draw(&mut display)
        .unwrap();

    let rect_style = PrimitiveStyle::with_fill(Rgb565::RED); // linie stanga
    Rectangle::new(Point::new(0, 0), Size::new(5, 240))
        .into_styled(rect_style)
        .draw(&mut display)
        .unwrap();

    let rect_style = PrimitiveStyle::with_fill(Rgb565::RED); //linie dreapta
    Rectangle::new(Point::new(315, 0), Size::new(5, 240))
        .into_styled(rect_style)
        .draw(&mut display)
        .unwrap();

    let rect_style = PrimitiveStyle::with_fill(Rgb565::RED); //linie bet sus
    Rectangle::new(Point::new(0, 200), Size::new(320, 5))
        .into_styled(rect_style)
        .draw(&mut display)
        .unwrap();

    let rect_style = PrimitiveStyle::with_fill(Rgb565::RED); //mijloc bet
    Rectangle::new(Point::new(155, 200), Size::new(5, 40))
        .into_styled(rect_style)
        .draw(&mut display)
        .unwrap();

    let text_style = MonoTextStyle::new(&FONT_10X20, Rgb565::GREEN);

    // Slot number 1
    let rect_style = PrimitiveStyle::with_fill(Rgb565::RED);
    Rectangle::new(Point::new(28, 58), Size::new(84, 84))
        .into_styled(rect_style)
        .draw(&mut display)
        .unwrap();

    let rect_style = PrimitiveStyle::with_fill(Rgb565::BLACK);
    Rectangle::new(Point::new(30, 60), Size::new(80, 80))
        .into_styled(rect_style)
        .draw(&mut display)
        .unwrap();

    // Slot number 2
    let rect_style = PrimitiveStyle::with_fill(Rgb565::RED);
    Rectangle::new(Point::new(118, 58), Size::new(84, 84))
        .into_styled(rect_style)
        .draw(&mut display)
        .unwrap();

    let rect_style = PrimitiveStyle::with_fill(Rgb565::BLACK);
    Rectangle::new(Point::new(120, 60), Size::new(80, 80))
        .into_styled(rect_style)
        .draw(&mut display)
        .unwrap();

    // Slot number 3
    let rect_style = PrimitiveStyle::with_fill(Rgb565::RED);
    Rectangle::new(Point::new(208, 58), Size::new(84, 84))
        .into_styled(rect_style)
        .draw(&mut display)
        .unwrap();

    let rect_style = PrimitiveStyle::with_fill(Rgb565::BLACK);
    Rectangle::new(Point::new(210, 60), Size::new(80, 80))
        .into_styled(rect_style)
        .draw(&mut display)
        .unwrap();

    let seed = Instant::now().as_ticks() as u64;
    let mut rng = SmallRng::seed_from_u64(seed);

    let slot_positions = [Point::new(30, 60), Point::new(120, 60), Point::new(210, 60)];
    let slot_size = Size::new(80, 80);

    let mut last_colors = [Rgb565::BLACK; 3];
    let mut win_amount = 0;
    let mut bet = 500;
    // let mut balance = 10000;
    let mut cashout=1;
    let mut publ = CHANNEL.publisher().unwrap();

    loop{
        let mut buffer: String<32> = String::new();
        write!(&mut buffer, "BALANCE:{}", BALANCE.load(Ordering::SeqCst)).unwrap();
        // write!(&mut buffer, "BALANCE: {}", balance).unwrap();
        Text::new(&buffer, Point::new(10, 225), text_style)
            .draw(&mut display)
            .unwrap();

        if increase_bet.is_low() {
            Rectangle::new(Point::new(220, 210), Size::new(70, 20))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                .draw(&mut display)
                .unwrap();
            if bet < 2500 {
                bet += 500;
            }
            else{
                bet=500;
            }
            publ.publish(State::BET).await;
        }
        if max_bet.is_low() {
            Rectangle::new(Point::new(220, 210), Size::new(70, 20))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                .draw(&mut display)
                .unwrap();
            bet=2500;
            publ.publish(State::BET).await;
        }
        let mut buffer: String<32> = String::new();
        write!(&mut buffer, "BET: {}", bet).unwrap();
        Text::new(&buffer, Point::new(170, 225), text_style)
            .draw(&mut display)
            .unwrap();

        if cashout_button.is_low() {

            cashout+=1;
    
            if cashout%2==0{

                publ.publish(State::ADDBALANCE).await;

                info!("Adding balance");
                Rectangle::new(Point::new(90, 210), Size::new(60, 20))
                    .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                    .draw(&mut display)
                    .unwrap();
                Timer::after_millis(50).await;
                let mut buffer: String<32> = String::new();
                write!(&mut buffer, "BALANCE:{}", BALANCE.load(Ordering::SeqCst)).unwrap();
                Text::new(&buffer, Point::new(10, 225), text_style)
                    .draw(&mut display)
                    .unwrap();
            }
            else{

                publ.publish(State::CASHOUT).await;

                info!("Cashout");
                Rectangle::new(Point::new(90, 210), Size::new(60, 20))
                    .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                    .draw(&mut display)
                    .unwrap();
                let mut buffer: String<32> = String::new();
                // BALANCE.store(0, Ordering::SeqCst);
                Timer::after_millis(50).await;
                write!(&mut buffer, "BALANCE:{}", BALANCE.load(Ordering::SeqCst)).unwrap();
                Text::new(&buffer, Point::new(10, 225), text_style)
                    .draw(&mut display)
                    .unwrap();
            }
        }


        if spin_button.is_low() {
                    
            if bet>BALANCE.load(Ordering::SeqCst){
                info!("Not enough money");
                Rectangle::new(Point::new(40, 15), Size::new(250, 40))
                    .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                    .draw(&mut display)
                    .unwrap();
                let text_style = MonoTextStyle::new(&FONT_10X20, Rgb565::RED);
                Text::new("Not enough money!", Point::new(80, 40), text_style)
                    .draw(&mut display)
                    .unwrap();
                Timer::after_millis(2000).await;
                Rectangle::new(Point::new(40, 15), Size::new(250, 40))
                    .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                    .draw(&mut display)
                    .unwrap();
            }
            else{

                publ.publish(State::SPIN).await;

                BALANCE.fetch_sub(bet, Ordering::SeqCst);

                Rectangle::new(Point::new(90, 210), Size::new(60, 20))
                    .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                    .draw(&mut display)
                    .unwrap();

                let mut buffer: String<32> = String::new();
                write!(&mut buffer, "BALANCE:{}", BALANCE.load(Ordering::SeqCst)).unwrap();
                Text::new(&buffer, Point::new(10, 225), text_style)
                    .draw(&mut display)
                    .unwrap();

                let mut buffer: String<32> = String::new();
                write!(&mut buffer, "LAST WIN: {}", win_amount).unwrap();
                Text::new(&buffer, Point::new(80, 175), text_style)
                    .draw(&mut display)
                    .unwrap();

                    //sterge mesajul de dupa fiecare rotire
                // Rectangle::new(Point::new(60, 15), Size::new(230, 40))
                //     .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                //     .draw(&mut display)
                //     .unwrap();


                for _ in 0..10 {
                    let predefined_colors = [
                        Rgb565::RED,
                        Rgb565::GREEN,
                        Rgb565::BLUE,
                        Rgb565::YELLOW,
                        Rgb565::CYAN,
                    ];

                    let mut colors = [Rgb565::BLACK; 3];

                    for i in 0..3 {
                        let color_index = rng.gen_range(0..predefined_colors.len());
                        colors[i] = predefined_colors[color_index];

                        let rect_style = PrimitiveStyle::with_fill(colors[i]);
                        Rectangle::new(slot_positions[i], slot_size)
                            .into_styled(rect_style)
                            .draw(&mut display)
                            .unwrap();
                    }

                    last_colors = colors;

                    Timer::after_millis(250).await;
                }

                let mut you_won = false;
                if last_colors[0] == last_colors[1] || last_colors[1] == last_colors[2] {
                    you_won = true;
                    win_amount = 10*(bet/10);
                    BALANCE.fetch_add(win_amount, Ordering::SeqCst);
                }

                if you_won {

                    publ.publish(State::WIN).await;

                        //modificarea balantei
                    Rectangle::new(Point::new(90, 210), Size::new(60, 20))
                        .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                        .draw(&mut display)
                        .unwrap();

                    Rectangle::new(Point::new(90, 15), Size::new(180, 40))
                        .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                        .draw(&mut display)
                        .unwrap();

                    let text_style = MonoTextStyle::new(&FONT_10X20, Rgb565::GREEN);
                    Text::new("YOU WON!!!", Point::new(110, 40), text_style)
                        .draw(&mut display)
                        .unwrap();

                    Rectangle::new(Point::new(180, 155), Size::new(70, 30))
                        .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                        .draw(&mut display)
                        .unwrap();

                    let mut buffer: String<32> = String::new();
                    write!(&mut buffer, "LAST WIN: {}", win_amount).unwrap();
                    Text::new(&buffer, Point::new(80, 175), text_style)
                        .draw(&mut display)
                        .unwrap();

                    info!("You won!");
                }
                else{
                    let messages = ["Strapped for cash!", "That hurts!", "Keep spinning!", "Almost there!", "Spent!", "Ruined!", "Bankrupt!", "Broke!", "Worthless!", "Soup line!"];
                    let message_index = rng.gen_range(0..messages.len());
                    let mut buffer: String<32> = String::new();

                    Rectangle::new(Point::new(80, 15), Size::new(200, 40))
                        .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                        .draw(&mut display)
                        .unwrap();

                    write!(&mut buffer, "{}", messages[message_index]).unwrap();
                    Text::new(&buffer, Point::new(90, 40), text_style)
                        .draw(&mut display)
                        .unwrap();
                }

                info!("Slot animation finished");
            }
        }

        Timer::after_millis(100).await;
    }
}

#[embassy_executor::task]
async fn led_task(
    mut yellow: Output<'static>,
    mut green: Output<'static>,
    mut blue: Output<'static>,
    mut red: Output<'static>,
) {
    info!("LED task started.");
    yellow.set_high();
    green.set_high();
    blue.set_high();
    red.set_high();

    let mut subs = CHANNEL.subscriber().unwrap();
    let mut subs_info = CHANNEL.subscriber().unwrap();

    loop {

        info!("Received value: {:?}", subs_info.next_message().await);

        match subs.next_message().await {
            wrm(State::SPIN) => {
                let start_time = embassy_time::Instant::now();
                info!("LED sequence started.");

                while embassy_time::Instant::now() - start_time
                    < embassy_time::Duration::from_millis(4000)
                {
                    yellow.set_high();
                    green.set_low();
                    blue.set_low();
                    red.set_low();
                    Timer::after_millis(200).await;

                    yellow.set_low();
                    green.set_high();
                    blue.set_low();
                    red.set_low();
                    Timer::after_millis(200).await;

                    yellow.set_low();
                    green.set_low();
                    blue.set_high();
                    red.set_low();
                    Timer::after_millis(200).await;

                    yellow.set_low();
                    green.set_low();
                    blue.set_low();
                    red.set_high();
                    Timer::after_millis(200).await;
                }

                info!("LED sequence finished, turning LEDs back on.");

                yellow.set_high();
                green.set_high();
                blue.set_high();
                red.set_high();
            }
            wrm(State::WIN) => {
                let start_time = embassy_time::Instant::now();

                while embassy_time::Instant::now() - start_time
                    < embassy_time::Duration::from_millis(1500)
                {
                    yellow.set_high();
                    green.set_high();
                    blue.set_high();
                    red.set_high();
                    Timer::after_millis(250).await;

                    yellow.set_low();
                    green.set_low();
                    blue.set_low();
                    red.set_low();
                    Timer::after_millis(250).await;
                }

                info!("LED sequence finished, turning LEDs back on.");

                yellow.set_high();
                green.set_high();
                blue.set_high();
                red.set_high();
            }
            wrm(State::BET) =>{}
            wrm(State::ADDBALANCE) =>{}
            wrm(State::CASHOUT) =>{}
            Lagged(_) => {}
        }
        Timer::after(Duration::from_millis(50)).await;
    }
}

#[embassy_executor::task]
async fn buzzer_task(mut buzzer: Pwm<'static>) {
    info!("Buzzer task started.");

    let mut config: ConfigPwm = Default::default();
    config.top = 5000;
    config.divider = 125_i32.to_fixed();
    config.compare_a = config.top / 2;

    buzzer.set_config(&config);
    buzzer.set_duty_cycle(0);

    let mut subs = CHANNEL.subscriber().unwrap();
    let mut subs_info = CHANNEL.subscriber().unwrap();

    loop {

        match subs.next_message().await {
            wrm(State::SPIN) => {
                for _ in 0..25 {
                    buzzer.set_duty_cycle(config.top / 2);
                    Timer::after(Duration::from_millis(50)).await;
                    buzzer.set_duty_cycle(0);
                    Timer::after(Duration::from_millis(50)).await;
                }
            }
            wrm(State::WIN) => {
                buzzer.set_duty_cycle(config.top / 2);
                Timer::after(Duration::from_millis(50)).await;
                buzzer.set_duty_cycle(0);
                Timer::after(Duration::from_millis(50)).await;
                buzzer.set_duty_cycle(config.top / 2);
                Timer::after(Duration::from_millis(1000)).await;
                buzzer.set_duty_cycle(0);
            }
            wrm(State::BET) => {
                buzzer.set_duty_cycle(config.top / 2);
                Timer::after(Duration::from_millis(50)).await;
                buzzer.set_duty_cycle(0);
                Timer::after(Duration::from_millis(50)).await;
            }
            wrm(State::ADDBALANCE) => {
                buzzer.set_duty_cycle(config.top / 2);
                Timer::after(Duration::from_millis(50)).await;
                buzzer.set_duty_cycle(0);
                Timer::after(Duration::from_millis(50)).await;
            }
            wrm(State::CASHOUT) => {
                buzzer.set_duty_cycle(config.top / 2);
                Timer::after(Duration::from_millis(50)).await;
                buzzer.set_duty_cycle(0);
                Timer::after(Duration::from_millis(50)).await;
            }
            Lagged(_) => {}
        }
        Timer::after(Duration::from_millis(50)).await;
    }
}


#[embassy_executor::task]
async fn rfid_task(
    spi: Spi<'static, SPI1, Blocking>,
    mut cs: Output<'static>,
    // mut reset: Output<'static>,
    mut i2c: I2c<'static, I2C1, I2cAsync>,
) {
    let mut mfrc = Mfrc522::new(spi).with_nss(cs).init().unwrap();

    let mut known_uids: [([u8; 4], u32); 2] = [
        // ([0xAA, 0xBB, 0xCC, 0xDD], 1), // Existing example UID with associated number
        (UID1, BALANCE1.load(Ordering::SeqCst) as u32),   // New known UID with associated number
        (UID2, BALANCE2.load(Ordering::SeqCst) as u32),      // Another known UID with associated number
    ];

    let mut subs = CHANNEL.subscriber().unwrap();

    loop {

        match subs.next_message().await {
            wrm(State::ADDBALANCE) => {
                let mut uid_bytes = [0u8; 4]; // Initialize uid_bytes with a default value

                match mfrc.new_card_present() {
                    Ok(atqa) => {
                        if let Ok(uid) = mfrc.select(&atqa) {
                            let uid_bytes = uid.as_bytes();
                            info!("Card UID: {:?}", uid_bytes);

                            let is_known = known_uids.iter().any(|&(k, _)| k == uid_bytes);
                            if is_known {
                                info!("Known card detected!");
                                for &mut (ref known_uid, ref mut number) in &mut known_uids {
                                    if known_uid == uid_bytes {
                                        BALANCE.store(*number as i32, Ordering::SeqCst);
                                        info!("Updated associated number: {}", BALANCE.load(Ordering::SeqCst));
                                        break;
                                    }
                                }
                            } else {
                                info!("Unknown card detected!");
                            }
                        }
                    }
                    Err(e) => {
                        // info!("Error checking for new card: {:?}", e);
                    }
                }
            }
            wrm(State::CASHOUT) => {
                let mut uid_bytes = [0u8; 4]; // Initialize uid_bytes with a default value

                match mfrc.new_card_present() {
                    Ok(atqa) => {
                        if let Ok(uid) = mfrc.select(&atqa) {
                            let uid_bytes = uid.as_bytes();
                            info!("Card UID: {:?}", uid_bytes);

                            let is_known = known_uids.iter().any(|&(k, _)| k == uid_bytes);
                            if is_known {
                                info!("Known card detected!");
                                for &mut (ref known_uid, ref mut number) in &mut known_uids {
                                    if known_uid == uid_bytes {
                                        *number = BALANCE.load(Ordering::SeqCst) as u32;

                                        if uid_bytes == UID1 {
                                            BALANCE1.store(BALANCE.load(Ordering::SeqCst), Ordering::SeqCst);
                                            info!("UID1 matched and BALANCE1 updated.");
                                        }
                                        else if uid_bytes == UID2 {
                                            BALANCE2.store(BALANCE.load(Ordering::SeqCst), Ordering::SeqCst);
                                            info!("UID2 matched and BALANCE2 updated.");
                                        }

                                        info!("Updated associated number: {}", BALANCE.load(Ordering::SeqCst));
                                        BALANCE.store(0, Ordering::SeqCst);

                                        write_card_data(&mut i2c).await.unwrap();
                                        break;
                                    }
                                }
                            } else {
                                info!("Unknown card detected!");
                            }
                        }
                    }
                    Err(e) => {
                        // info!("Error checking for new card: {:?}", e);
                    }
                }
            }
            wrm(State::SPIN) => {}
            wrm(State::WIN) => {}
            wrm(State::BET) => {}
            Lagged(_) => {}
        }

        Timer::after_millis(50).await;
    }
}


#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let yellow = Output::new(p.PIN_2, Level::Low);
    let green = Output::new(p.PIN_3, Level::Low);
    let blue = Output::new(p.PIN_4, Level::Low);
    let red = Output::new(p.PIN_5, Level::Low);
    let spin_button = Input::new(p.PIN_6, Pull::Up);
    let increase_bet = Input::new(p.PIN_7, Pull::Up);
    let max_bet = Input::new(p.PIN_8, Pull::Up);
    let cashout_button = Input::new(p.PIN_9, Pull::Up);

    let mut spiconfig1 = ConfigSpi::default();
    spiconfig1.frequency = 32_000_000;

    let miso1 = p.PIN_16;
    let mosi1 = p.PIN_19;
    let clk1 = p.PIN_18;

    let mut spi = Spi::new_blocking(p.SPI0, clk1, mosi1, miso1, spiconfig1);
    let spi_bus = NoopMutex::new(RefCell::new(spi));
    let spi_bus = SPI_BUS.init(spi_bus); // for sending to task

    let mut cs = Output::new(p.PIN_17, Level::High);
    let mut dc = Output::new(p.PIN_14, Level::Low);
    let mut reset = Output::new(p.PIN_15, Level::High);


    //rfid
    let miso2 = p.PIN_12;
    let mosi2 = p.PIN_11;
    let sck = p.PIN_10; 
    let rst = p.PIN_21;
    let sda = p.PIN_13;

    let mut cs2 = Output::new(sda, Level::Low);
    let mut reset2 = Output::new(rst, Level::High);
    reset2.set_low();
    Timer::after_millis(10).await;
    reset2.set_high();

    let mut spi_config2 = embassy_rp::spi::Config::default();
    spi_config2.frequency = 1_000_000;
    spi_config2.polarity = embassy_rp::spi::Polarity::IdleLow;
    spi_config2.phase = embassy_rp::spi::Phase::CaptureOnFirstTransition;

    let mut spi2 = Spi::new_blocking(p.SPI1, sck, mosi2, miso2, spi_config2);

    //memory
    let sda3 = p.PIN_26;
    let scl3 = p.PIN_27;
    let mut i2c = I2c::new_async(p.I2C1, scl3, sda3, Irqs, I2cConfig::default());

    // write_card_data(&mut i2c).await.unwrap();

    load_card_data(&mut i2c).await.unwrap();


    spawner.spawn(display_task(spi_bus, cs, dc, reset, increase_bet, max_bet, spin_button, cashout_button)).unwrap();
    spawner.spawn(led_task(yellow, green, blue, red)).unwrap();
    spawner.spawn(buzzer_task(Pwm::new_output_a(p.PWM_SLICE3, p.PIN_22, ConfigPwm::default()))).unwrap();
    spawner.spawn(rfid_task(spi2, cs2, i2c)).unwrap();


} 
