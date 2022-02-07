#![no_std]
#![no_main]
#![feature(bench_black_box)]

use cortex_m_rt::entry;
use hal::{prelude::*, gpio::Edge};
use panic_halt as _;
use stm32f4xx_hal as hal;

#[entry]
fn main() -> ! {
    let mut device = hal::pac::Peripherals::take().unwrap();
    let core = cortex_m::Peripherals::take().unwrap();

    let rcc = device.RCC.constrain();
    let clocks = rcc
        .cfgr
        .use_hse(26.mhz())
        .require_pll48clk()
        .sysclk(84.mhz())
        .hclk(84.mhz())
        .pclk1(21.mhz())
        .pclk2(42.mhz())
        .freeze();
    
    let mut syscfg = device.SYSCFG.constrain();

    // Initialize the different pins
    let gpioc = device.GPIOC.split();
    let gpiod = device.GPIOD.split();
    let gpiob = device.GPIOB.split();

    let cs = gpiod.pd2.into_push_pull_output();
    let mut data_int = gpiob.pb3.into_pull_up_input();
    data_int.make_interrupt_source(&mut syscfg);
    data_int.enable_interrupt(&mut device.EXTI);
    data_int.trigger_on_edge(&mut device.EXTI, Edge::Falling);

    let ce = gpiod.pd3.into_push_pull_output();

    let mosi = gpioc.pc12.into_alternate();
    let miso = gpioc.pc11.into_alternate();
    let sclk = gpioc.pc10.into_alternate();

    let spi = hal::spi::Spi::new(
        device.SPI3,
        (sclk, miso, mosi),
        nrf24_rs::SPI_MODE,
        1.mhz(),
        &clocks,
    );

    let mut delay = hal::delay::Delay::new(core.SYST, &clocks);

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
    nrf_chip.open_reading_pipe(nrf24_rs::config::DataPipe::DP0, b"Node1").unwrap();

    nrf_chip.start_listening().unwrap();

    loop {
        while !nrf_chip.data_available().unwrap() {
            delay.delay_ms(50u16);
        }
        let mut buffer = [0u8; nrf24_rs::MAX_PAYLOAD_SIZE as usize];
        nrf_chip.read(&mut buffer).unwrap();

        loop {}
    }
}
