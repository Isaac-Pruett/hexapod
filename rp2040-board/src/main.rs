//! This example shows how to create a pwm using the PIO module in the RP2040 chip.

#![no_std]
#![no_main]
use core::time::Duration;
use embassy_rp::pac::dma::vals::DataSize;
use embassy_rp::pio_programs::ws2812::Grb;
use embassy_rp::pio_programs::ws2812::PioWs2812;
use embassy_rp::pio_programs::ws2812::PioWs2812Program;

use crate::servos::AllServos;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::Output;
use embassy_rp::peripherals::{PIO0, PIO1};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::pio_programs::pwm::{PioPwm, PioPwmProgram};
use embassy_time::Timer;
use smart_leds::RGB8;
use {defmt_rtt as _, panic_probe as _};

mod servos;
use servos::ServoFromPIO;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
    PIO1_IRQ_0 => InterruptHandler<PIO1>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    // Take the NeoPixel-related peripherals explicitly

    let pin_leds = p.PIN_18;

    let Pio {
        mut common, sm0, ..
    } = Pio::new(p.PIO1, Irqs);

    // This is the number of leds in the string. Helpfully, the sparkfun thing plus and adafruit
    // feather boards for the 2040 both have one built in.
    const NUM_LEDS: usize = 6;
    let mut data = [RGB8::default(); NUM_LEDS];

    // Common neopixel pins:
    // Thing plus: 8
    // Adafruit Feather: 16;  Adafruit Feather+RFM95: 4
    let program = PioWs2812Program::new(&mut common);
    let mut ws2812: PioWs2812<'_, PIO1, 0, NUM_LEDS, Grb> =
        PioWs2812::new(&mut common, sm0, p.DMA_CH0, pin_leds, &program);

    data[0] = RGB8::new(20, 127, 255);
    ws2812.write(&data).await;

    let mut all_servos = AllServos::new(
        p.PIO0,
        p.PWM_SLICE0,
        p.PWM_SLICE1,
        p.PWM_SLICE2,
        p.PWM_SLICE3,
        p.PWM_SLICE4,
        p.PWM_SLICE5,
        p.PWM_SLICE6,
        p.PWM_SLICE7,
        p.PIN_0,
        p.PIN_1,
        p.PIN_2,
        p.PIN_3,
        p.PIN_4,
        p.PIN_5,
        p.PIN_6,
        p.PIN_7,
        p.PIN_8,
        p.PIN_9,
        p.PIN_10,
        p.PIN_11,
        p.PIN_12,
        p.PIN_13,
        p.PIN_14,
        p.PIN_15,
        p.PIN_16,
        p.PIN_17,
    );

    all_servos.zero_all();
    Timer::after_millis(8000).await;

    let mut servos = all_servos.servos;

    loop {
        // s1.to_max().await;

        servos[0].write(0.0);
        servos[17].write(0.0);

        Timer::after_millis(2000).await;

        servos[0].write(90.0);
        servos[17].write(90.0);

        Timer::after_millis(2000).await;

        servos[0].write(180.0);
        servos[17].write(180.0);

        Timer::after_millis(2000).await;
        // Timer::after_millis(300).await;
        // s1.to_min().await;
        // Timer::after_millis(300).await;
    }
}
