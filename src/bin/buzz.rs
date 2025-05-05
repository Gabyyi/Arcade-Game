#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::{
    gpio::{Input, Level, Output, Pull},
    peripherals::{SPI0, SPI1},
    pwm::{Config as ConfigPwm, Pwm, SetDutyCycle},
    spi::{Blocking, Config as ConfigSpi, Spi},
};
use embassy_sync::{blocking_mutex::NoopMutex, channel};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, channel::Channel};
use embassy_time::{Delay, Duration, Timer};
// use heapless::mpmc::{Q64, Consumer, Producer};
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

static CHANNEL: PubSubChannel<ThreadModeRawMutex, State, 1000, 4, 4> = PubSubChannel::new();

#[derive(Clone, Copy, PartialEq, defmt::Format)]
enum State {
    SPIN,
    WIN,
    BET,
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
    let mut bet = 10;
    let mut balance = 10000;
    let mut publ = CHANNEL.publisher().unwrap();


    loop {
        let mut buffer: String<32> = String::new();
        write!(&mut buffer, "BALANCE: {}", balance).unwrap();
        Text::new(&buffer, Point::new(10, 225), text_style)
            .draw(&mut display)
            .unwrap();

        if increase_bet.is_low() {
            Rectangle::new(Point::new(220, 210), Size::new(70, 20))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                .draw(&mut display)
                .unwrap();
            if bet < 50 {
                bet += 10;
            }
            else{
                bet=10;
            }
            publ.publish(State::BET).await;
        }
        if max_bet.is_low() {
            Rectangle::new(Point::new(220, 210), Size::new(70, 20))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                .draw(&mut display)
                .unwrap();
            bet=50;
            publ.publish(State::BET).await;
        }
        let mut buffer: String<32> = String::new();
        write!(&mut buffer, "BET: {}", bet).unwrap();
        Text::new(&buffer, Point::new(170, 225), text_style)
            .draw(&mut display)
            .unwrap();


        if spin_button.is_low() {

            publ.publish(State::SPIN).await;

            info!("Spin button pressed, starting slot animation.");

            balance = balance - bet;
            Rectangle::new(Point::new(100, 210), Size::new(50, 20))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                .draw(&mut display)
                .unwrap();

            let mut buffer: String<32> = String::new();
            write!(&mut buffer, "BALANCE: {}", balance).unwrap();
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
                // win_amount = rng.gen_range(0..100);
                win_amount = 10*(bet/10);
                balance += win_amount;
            }

            if you_won {

                publ.publish(State::WIN).await;

                    //modificarea balantei
                Rectangle::new(Point::new(100, 210), Size::new(50, 20))
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
            Lagged(_) => {}
        }
        Timer::after(Duration::from_millis(50)).await;

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

    spawner.spawn(display_task(spi_bus, cs, dc, reset, increase_bet, max_bet, spin_button)).unwrap();
    spawner.spawn(led_task(yellow, green, blue, red)).unwrap();
    spawner.spawn(buzzer_task(Pwm::new_output_a(p.PWM_SLICE3, p.PIN_22, ConfigPwm::default()))).unwrap();


}
