#![no_std]
#![no_main]
#![feature(bench_black_box)]

use cortex_m_rt::entry;
use hal::{prelude::*, gpio::Edge};
use panic_halt as _;
use stm32f1xx_hal as hal;

#[entry]
fn main() -> ! {
    let dp = hal::pac::Peripherals::take().unwrap();
    let core = cortex_m::Peripherals::take().unwrap();
    let mut flash = dp.FLASH.constrain();
    let mut afio = dp.AFIO.constrain();

    let rcc = dp.RCC.constrain();
    let clocks = rcc
        .cfgr
        .use_hse(8.mhz())
        .sysclk(72.mhz())
        .hclk(72.mhz())
        .freeze(&mut flash.acr);
    
    // Initialize the different pins
    let mut gpioa = dp.GPIOA.split();

    let cs = gpioa.pa4.into_push_pull_output(&mut gpioa.crl);
    //let mut data_int = gpiob.pb3.into_pull_up_input();
    //data_int.make_interrupt_source(&mut syscfg);
    //data_int.enable_interrupt(&mut device.EXTI);
    //data_int.trigger_on_edge(&mut device.EXTI, Edge::Falling);

    let ce = gpioa.pa3.into_push_pull_output(&mut gpioa.crl);

    let mosi = gpioa.pa7.into_alternate_push_pull(&mut gpioa.crl);
    let miso = gpioa.pa6;
    let sclk = gpioa.pa5.into_alternate_push_pull(&mut gpioa.crl);

    let spi = hal::spi::Spi::spi1(
        dp.SPI1,
        (sclk, miso, mosi),
        &mut afio.mapr,
        nrf24_rs::SPI_MODE,
        1.mhz(),
        clocks,
    );

    let mut delay = hal::delay::Delay::new(core.SYST, clocks);

    let message = b"Hello world!"; // The message we will be sending

    // Setup some configuration values
    let config = nrf24_rs::config::NrfConfig::default()
        .channel(8)
        .pa_level(nrf24_rs::config::PALevel::Min)
        // We will use a payload size the size of our message
        .payload_size(nrf24_rs::MAX_PAYLOAD_SIZE);

    // Initialize the chip
    let mut nrf_chip = nrf24_rs::Nrf24l01::new(spi, ce, cs, &mut delay, config).unwrap();
    if !nrf_chip.is_connected().unwrap() {
        panic!("Chip is not connected.");
    }

    // Open a writing pipe on address "Node1".
    // The listener will have to open a reading pipe with the same address
    // in order to receive this message.
    nrf_chip.open_writing_pipe(b"Node1").unwrap();

    // Keep trying to send the message
    while let Err(e) = nrf_chip.write(&mut delay, message) {
        // Something went wrong while writing, try again in 50ms
        delay.delay_ms(50u16);
    }

    // Message should now successfully have been sent!
    loop {}
}
