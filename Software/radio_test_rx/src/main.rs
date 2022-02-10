#![no_std]
#![no_main]
#![feature(bench_black_box)]

use core::panic::PanicInfo;
use cortex_m_rt::entry;
use hal::{
    device::gpioa::CRH,
    gpio::{Output, Pin, PushPull},
    prelude::*,
};
use stm32f1xx_hal as hal;

static mut PANIC_LED: *mut Pin<Output<PushPull>, CRH, 'C', 13> = core::ptr::null_mut();

#[panic_handler]
fn panic_handler(_: &PanicInfo) -> ! {
    // We assume that `PANIC_LED` has been set
    let led = unsafe { PANIC_LED };
    if led.is_null() {
        loop {}
    } else {
        // SAFETY: led is non null, so it must have been initialized
        let led = unsafe { &mut *PANIC_LED };
        loop {
            for _ in 0..1000 {
                cortex_m::asm::delay(70_200_000);
                // 72_000_000 / 7_200_000 ~= 10Hz
            }
            led.toggle();
        }
    }
}

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
    let mut gpioc = dp.GPIOC.split();

    let mut led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);
    led.set_high();

    // So so unsafe. I'm not gonna try to make a safety argument here because so many things are wrong
    // This works on the current compiler to allow us to easily control the LED from the panic
    // handler
    let led_ptr: *mut Pin<Output<PushPull>, CRH, 'C', 13> =
        unsafe { core::mem::transmute(&mut led) };
    unsafe { PANIC_LED = led_ptr };

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

    // Setup some configuration values
    let config = nrf24_rs::config::NrfConfig::default()
        .channel(8)
        .pa_level(nrf24_rs::config::PALevel::Max)
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
            // No data availble, wait 50ms, then check again
            delay.delay_ms(50u16);
        }
        led.toggle();
        // Now there is some data availble to read

        // Initialize empty buffer
        let mut buffer = [0u8; nrf24_rs::MAX_PAYLOAD_SIZE as usize];
        nrf_chip.read(&mut buffer).unwrap();
        delay.delay_ms(125u16);
        led.toggle();
    }
}
