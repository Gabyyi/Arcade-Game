#![no_std]
#![no_main]

extern crate alloc;

use embassy_executor::Spawner;
use embassy_rp::{gpio::{Input, Level, Output, Pull}, peripherals::{PIN_20, SPI0, SPI1}, spi::{Blocking, Config as ConfigSpi, Spi}, usb::Out};
use embassy_time::{Delay, Timer};
use embassy_sync::blocking_mutex::NoopMutex;
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDevice;
use defmt::info;

// Add this code to Cargo.toml (modified version of st7735-lcd-rs crate)
// [dependencies]
// ili9341 = { version = "0.6.1", git = "https://github.com/mohgTheOmen/ili9341-rs" }
// static_cell = "1.2"
// display-interface-spi = "0.5.0"
//
// [dependencies.embedded-graphics]
// version = "0.7"
// optional = true
//
// [features]
// default = ["graphics"]
// graphics = ["embedded-graphics"]

use st7735_lcd::{Orientation as OrientationST7735, ST7735};
use ili9341::{Ili9341, Orientation as OrientationIli9341, DisplaySize240x320, ModeState};
use core::cell::RefCell;
use alloc::vec::Vec;
use display_interface_spi::SPIInterface;
// use core::cell::RefCell;
use static_cell::StaticCell;
use embedded_graphics::{mono_font::{ascii::FONT_10X20, MonoTextStyle}, pixelcolor::Rgb565, prelude::*, text::Text, primitives::{PrimitiveStyle, Circle, Rectangle}};
use embedded_canvas::Canvas;
use embedded_graphics::Pixel;
use {defmt_rtt as _, panic_probe as _};


static SPI_BUS: StaticCell<NoopMutex<RefCell<Spi<'static, SPI0, Blocking>>>> = StaticCell::new();    // for borrowing to a task


#[embassy_executor::task]
async fn dispay_task(
    spi_bus: &'static NoopMutex<RefCell<Spi<'static, SPI0, Blocking>>>,
    mut cs: Output<'static>,
    mut dc: Output<'static>,
    mut reset: Output<'static>,
    mut cs2: Output<'static>,
    mut dc2: Output<'static>,
    mut reset2: Output<'static>
) {
    let spi_dev = SpiDevice::new(&spi_bus, cs);
    let iface = SPIInterface::new(spi_dev, dc);

    let mut delay = Delay;

    let mut display = Ili9341::new(iface, reset, &mut delay, OrientationIli9341::LandscapeFlipped, DisplaySize240x320).unwrap();

    display.idle_mode(ModeState::Off).unwrap();
    display.invert_mode(ModeState::On).unwrap();
    let _ = display.normal_mode_frame_rate(ili9341::FrameRateClockDivision::Fosc, ili9341::FrameRate::FrameRate100);
    display.clear(Rgb565::BLACK).unwrap();




    // let spi_dev2 = SpiDevice::new(&spi_bus, cs2);

    // let mut delay2 = Delay;

    // let mut display2 = ST7735::new(spi_dev2, dc2, core::prelude::v1::Some(reset2), Default::default(), false, 132, 130);
    // display2.init(&mut delay2).unwrap();
    // display2.set_orientation(&OrientationST7735::Portrait).unwrap();

    
    // display2.clear(Rgb565::BLACK).unwrap();      //eroare la clear
    // display2.idle_mode(ModeState::Off).unwrap();
    // display2.invert_mode(ModeState::On).unwrap();
    // let _ = display2.normal_mode_frame_rate(ili9341::FrameRateClockDivision::Fosc, ili9341::FrameRate::FrameRate100);
    // display2.clear(Rgb565::BLACK).unwrap();

    // let text_style = MonoTextStyle::new(&FONT_10X20, Rgb565::GREEN);
    // Text::new("SECOND DISPLAY", Point::new(10, 225), text_style)
    //     .draw(&mut display2)      // eroare la display2
    //     .unwrap();




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
    Text::new("BALANCE: 10000", Point::new(10, 225), text_style)
        .draw(&mut display)
        .unwrap();

    let text_style = MonoTextStyle::new(&FONT_10X20, Rgb565::GREEN);
    Text::new("BET:   20", Point::new(170, 225), text_style)
        .draw(&mut display)
        .unwrap();


    let text_style = MonoTextStyle::new(&FONT_10X20, Rgb565::GREEN);
    Text::new("YOU WON!!!", Point::new(110, 40), text_style)
        .draw(&mut display)
        .unwrap();

    let text_style = MonoTextStyle::new(&FONT_10X20, Rgb565::GREEN);
    Text::new("AMOUNT WON: 10000", Point::new(70, 175), text_style)
        .draw(&mut display)
        .unwrap();

    // Slot number 1
    let rect_style = PrimitiveStyle::with_fill(Rgb565::RED);
    Rectangle::new(Point::new(30, 60), Size::new(80, 80))
        .into_styled(rect_style)
        .draw(&mut display)
        .unwrap();

    // Slot number 2
    let rect_style = PrimitiveStyle::with_fill(Rgb565::RED);
    Rectangle::new(Point::new(120, 60), Size::new(80, 80))
        .into_styled(rect_style)
        .draw(&mut display)
        .unwrap();

    // Slot number 3
    let rect_style = PrimitiveStyle::with_fill(Rgb565::RED);
    Rectangle::new(Point::new(210, 60), Size::new(80, 80))
        .into_styled(rect_style)
        .draw(&mut display)
        .unwrap();  

    info!("Display initialized and text written.");


    loop {
        Timer::after_millis(500).await;
    }
}


#[embassy_executor::task]
async fn led_task(mut yellow: Output<'static>, mut green: Output<'static>, mut blue: Output<'static>, mut red: Output<'static>, mut button: Input<'static>) {
    info!("LED task started.");
    yellow.set_high();
    green.set_high();
    blue.set_high();
    red.set_high();

    loop {
        button.wait_for_falling_edge().await;
        info!("Button pressed, starting LED sequence.");
        let start_time = embassy_time::Instant::now();

        while embassy_time::Instant::now() - start_time < embassy_time::Duration::from_millis(5000) {
            yellow.set_high();
            green.set_low();
            blue.set_low();
            red.set_low();
            Timer::after_millis(250).await;

            yellow.set_low();
            green.set_high();
            blue.set_low();
            red.set_low();
            Timer::after_millis(250).await;

            yellow.set_low();
            green.set_low();
            blue.set_high();
            red.set_low();
            Timer::after_millis(250).await;

            yellow.set_low();
            green.set_low();
            blue.set_low();
            red.set_high();
            Timer::after_millis(250).await;
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

    let mut cs2=Output::new(p.PIN_20, Level::High);
    let mut dc2=Output::new(p.PIN_21, Level::Low);
    let mut reset2=Output::new(p.PIN_22, Level::High);

    spawner.spawn(dispay_task(spi_bus, cs, dc, reset, cs2, dc2, reset2)).unwrap();
    spawner.spawn(led_task(yellow, green, blue, red, button)).unwrap();
    
}