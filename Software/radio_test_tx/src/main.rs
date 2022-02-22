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

    let message = b"Hello world! I need to fill all the bytes please!"; // The message we will be sending
    let message = &message[..nrf24_rs::MAX_PAYLOAD_SIZE as usize];


    use common::{IndexedBlock, MainCipher, KEY, Tag};

    // We also need an index key that is used to encrypt the index when sent in the clear
    let index_key: [u8; 4] = *include_bytes!("../../private/index-key.bin");
    let index_key = u32::from_ne_bytes(index_key);
    let cipher = MainCipher::new(&KEY, index_key);
     
    let mut block = IndexedBlock::new();
     
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
    nrf_chip.open_writing_pipe(b"Node1").unwrap();

    let mut index = 0;
    // Message should now successfully have been sent!
    loop {

        let bytes = block.as_bytes_mut();

        //Write message
        // let base_msg = b"This is a test message      ";
        // for (i, b) in base_msg.iter().enumerate() {
        //     bytes[i] = *b;
        // }
        //Write index
        for i in 0..7 {
            block.data_mut()[i] = i as u32;
        }
        block.tag().set_index(index);
        index += 1;

        block.do_cipher(&cipher);
        let message = block.as_bytes();
        assert_eq!(message.len() as u8, nrf24_rs::MAX_PAYLOAD_SIZE);

        // Keep trying to send the message
        while nrf_chip.write(&mut delay, message).is_err() {
            // Something went wrong while writing, try again in 50ms
            delay.delay_ms(50u16);
        }
        led.toggle();
        delay.delay_ms(200u16);
        led.toggle();
        delay.delay_ms(800u16);
    }
}
