#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::{gpio::{Level, Output, Input, Pull}, peripherals::{SPI0, SPI1}, spi::{Blocking, Config as ConfigSpi, Spi}};
use embassy_time::{Delay, Timer};
use embassy_sync::blocking_mutex::NoopMutex;
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDevice;
use defmt::info;
use ili9341::{DisplaySize240x320, DisplaySize320x480, Ili9341, ModeState, Orientation};
use display_interface_spi::SPIInterface;
use core::cell::RefCell;
use static_cell::StaticCell;
use embedded_graphics::{mono_font::{ascii::FONT_10X20, MonoTextStyle}, pixelcolor::Rgb565, prelude::*, text::Text, primitives::{PrimitiveStyle, Circle, Rectangle}};
use {defmt_rtt as _, panic_probe as _};
use core::fmt::Write; // Import for core formatting
use heapless::String; // Import for no_std string handling

static SPI_BUS: StaticCell<NoopMutex<RefCell<Spi<'static, SPI0, Blocking>>>> = StaticCell::new();    // for borrowing to a task


use embassy_sync::signal::Signal;
use rand::{Rng, SeedableRng};
use rand::rngs::SmallRng;
use embassy_time::Instant;


static SPIN_BUTTON_PRESSED: Signal<embassy_sync::blocking_mutex::raw::ThreadModeRawMutex, bool> = Signal::new();  // Global signal

#[embassy_executor::task]
async fn dispay_task(
    spi_bus: &'static NoopMutex<RefCell<Spi<'static, SPI0, Blocking>>>,
    mut cs: Output<'static>,
    mut dc: Output<'static>,
    mut reset: Output<'static>,
    mut increase_bet: Input<'static>,
    mut decrease_bet: Input<'static>
) {
    let spi_dev = SpiDevice::new(&spi_bus, cs);
    let iface = SPIInterface::new(spi_dev, dc);

    let mut delay = Delay;

    let mut display = Ili9341::new(iface, reset, &mut delay, Orientation::LandscapeFlipped, DisplaySize240x320).unwrap();

    display.idle_mode(ModeState::Off).unwrap();
    display.invert_mode(ModeState::On).unwrap();
    let _ = display.normal_mode_frame_rate(ili9341::FrameRateClockDivision::Fosc, ili9341::FrameRate::FrameRate100);
    display.clear(Rgb565::BLACK).unwrap();


    let rect_style = PrimitiveStyle::with_fill(Rgb565::RED);    //linie sus
    Rectangle::new(Point::new(0, 0), Size::new(320, 5))
        .into_styled(rect_style)
        .draw(&mut display)
        .unwrap();

    let rect_style = PrimitiveStyle::with_fill(Rgb565::RED);    //linie jos
    Rectangle::new(Point::new(0, 235), Size::new(320, 5))
        .into_styled(rect_style)
        .draw(&mut display)
        .unwrap();

    let rect_style = PrimitiveStyle::with_fill(Rgb565::RED); // linie stanga
    Rectangle::new(Point::new(0, 0), Size::new(5, 240))
        .into_styled(rect_style)
        .draw(&mut display)
        .unwrap();

    let rect_style = PrimitiveStyle::with_fill(Rgb565::RED);  //linie dreapta
    Rectangle::new(Point::new(315, 0), Size::new(5, 240))
        .into_styled(rect_style)
        .draw(&mut display)
        .unwrap();

    let rect_style = PrimitiveStyle::with_fill(Rgb565::RED);    //linie bet sus
    Rectangle::new(Point::new(0, 200), Size::new(320, 5))
        .into_styled(rect_style)
        .draw(&mut display)
        .unwrap();
    
    let rect_style = PrimitiveStyle::with_fill(Rgb565::RED);   //mijloc bet
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

    let slot_positions = [
        Point::new(30, 60),
        Point::new(120, 60),
        Point::new(210, 60),
    ];
    let slot_size = Size::new(80, 80);

    let mut last_colors = [Rgb565::BLACK; 3];
    let mut win_amount = 0;
    let mut bet = 10;
    let mut balance = 10000;

    loop {

        let mut buffer: String<32> = String::new();
        write!(&mut buffer, "BALANCE: {}", balance).unwrap();
        Text::new(&buffer, Point::new(10, 225), text_style)
            .draw(&mut display)
            .unwrap();

        if increase_bet.is_low(){
            Rectangle::new(Point::new(220,210), Size::new(70, 20))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                .draw(&mut display)
                .unwrap();
            if bet<50{
                bet+=10;
            }
        }
        if decrease_bet.is_low(){
            Rectangle::new(Point::new(220, 210), Size::new(70, 20))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                .draw(&mut display)
                .unwrap();
            if bet>10{
                bet-=10;
            }
        }
        let mut buffer: String<32> = String::new();
        write!(&mut buffer, "BET: {}", bet).unwrap();
        Text::new(&buffer, Point::new(170, 225), text_style)
            .draw(&mut display)
            .unwrap();



        if SPIN_BUTTON_PRESSED.signaled() {
            // Reset signal
            SPIN_BUTTON_PRESSED.reset();

            balance=balance-bet;
            Rectangle::new(Point::new(100,210), Size::new(50, 20))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                .draw(&mut display)
                .unwrap();

            let mut buffer: String<32> = String::new();
            write!(&mut buffer, "BALANCE: {}", balance).unwrap();
            Text::new(&buffer, Point::new(10, 225), text_style)
                .draw(&mut display)
                .unwrap();

            Rectangle::new(Point::new(90, 6), Size::new(150, 40))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                .draw(&mut display)
                .unwrap();

            let mut buffer: String<32> = String::new();
            write!(&mut buffer, "LAST WIN: {}", win_amount).unwrap();
            Text::new(&buffer, Point::new(80, 175), text_style)
                .draw(&mut display)
                .unwrap();

            info!("Starting slot animation");

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
                win_amount = rng.gen_range(0..100);
                balance += win_amount;
            }

            if you_won {
                Rectangle::new(Point::new(100,210), Size::new(50, 20))
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
    mut button: Input<'static>
) {
    info!("LED task started.");
    yellow.set_high();
    green.set_high();
    blue.set_high();
    red.set_high();

    loop {
        button.wait_for_falling_edge().await;
        info!("Button pressed, starting LED sequence and slot animation.");
        
        SPIN_BUTTON_PRESSED.signal(true); // Signal the display task

        let start_time = embassy_time::Instant::now();

        while embassy_time::Instant::now() - start_time < embassy_time::Duration::from_millis(5000) {
            yellow.set_high();
            green.set_low();
            blue.set_low();
            red.set_low();
            Timer::after_millis(220).await;

            yellow.set_low();
            green.set_high();
            blue.set_low();
            red.set_low();
            Timer::after_millis(220).await;

            yellow.set_low();
            green.set_low();
            blue.set_high();
            red.set_low();
            Timer::after_millis(220).await;

            yellow.set_low();
            green.set_low();
            blue.set_low();
            red.set_high();
            Timer::after_millis(220).await;
        }

        info!("LED sequence finished, turning LEDs back on.");

        yellow.set_high();
        green.set_high();
        blue.set_high();
        red.set_high();
    }
}



#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let mut yellow = Output::new(p.PIN_2, Level::Low);
    let mut green = Output::new(p.PIN_3, Level::Low);
    let mut blue = Output::new(p.PIN_4, Level::Low);
    let mut red = Output::new(p.PIN_5, Level::Low);
    let button = Input::new(p.PIN_6, Pull::Up);
    let increase_bet = Input::new(p.PIN_7, Pull::Up);
    let decrease_bet = Input::new(p.PIN_8, Pull::Up);    
    let cashout_button=Input::new(p.PIN_9, Pull::Up);

    let mut spiconfig1 = ConfigSpi::default();
    spiconfig1.frequency = 32_000_000;

    let miso1 = p.PIN_16;
    let mosi1 = p.PIN_19;
    let clk1 = p.PIN_18;

    let mut spi = Spi::new_blocking(p.SPI0, clk1, mosi1, miso1, spiconfig1);
    let spi_bus = NoopMutex::new(RefCell::new(spi));
    let spi_bus = SPI_BUS.init(spi_bus);     // for sending to task

    let mut cs = Output::new(p.PIN_17, Level::High);
    let mut dc = Output::new(p.PIN_14, Level::Low);
    let mut reset = Output::new(p.PIN_15, Level::High);

    spawner.spawn(dispay_task(spi_bus, cs, dc, reset, increase_bet, decrease_bet)).unwrap();
    spawner.spawn(led_task(yellow, green, blue, red, button)).unwrap();
    
}