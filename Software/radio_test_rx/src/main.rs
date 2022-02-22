#![no_std]
#![no_main]
#![feature(bench_black_box)]

use core::panic::PanicInfo;
use cortex_m_rt::entry;
use hal::{
    device::gpioa::CRH,
    gpio::{Output, Pin, PushPull},
    prelude::*,
    usb::{Peripheral, UsbBus},
};
use usb_device::prelude::*;
use usbd_serial::{SerialPort, USB_CLASS_CDC};
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


    // BluePill board has a pull-up resistor on the D+ line.
    // Pull the D+ pin down to send a RESET condition to the USB bus.
    // This forced reset is needed only for development, without it host
    // will not reset your device when you upload new firmware.
    let mut usb_dp = gpioa.pa12.into_push_pull_output(&mut gpioa.crh);
    usb_dp.set_low();
    delay.delay_us(1u16);

    let usb = Peripheral {
        usb: dp.USB,
        pin_dm: gpioa.pa11,
        pin_dp: usb_dp.into_floating_input(&mut gpioa.crh),
    };
    let usb_bus = UsbBus::new(usb);

    let mut serial = SerialPort::new(&usb_bus);

    let mut usb_device = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
        .manufacturer("Fake Company")
        .product("Serial Port")
        .serial_number("TEST")
        .device_class(USB_CLASS_CDC)
        .build();

    use common::{IndexedBlock, MainCipher, KEY, Tag};

    // We also need an index key that is used to encrypt the index when sent in the clear
    let index_key: [u8; 4] = *include_bytes!("../../private/index-key.bin");
    let index_key = u32::from_ne_bytes(index_key);
    let cipher = MainCipher::new(&KEY, index_key);
     
    let mut block = IndexedBlock::new();     

    // Open a writing pipe on address "Node1".
    // The listener will have to open a reading pipe with the same address
    // in order to receive this message.
    nrf_chip.open_reading_pipe(nrf24_rs::config::DataPipe::DP0, b"Node1").unwrap();

    nrf_chip.start_listening().unwrap();

    loop {
        usb_device.poll(&mut [&mut serial]);

        delay.delay_us(2u16);
        let msg = "TEST _ A ";
        let buf = msg.as_bytes();
        let mut write_offset = 0;
        while write_offset < buf.len() {
            match serial.write(&buf[write_offset..]) {
                Ok(len) if len > 0 => {
                    write_offset += len;
                }
                _ => {}
            }
        }

        led.toggle();
    }

    let mut x = 0;
    loop {

        x += 1;
        if x == 10_000 {
            led.toggle();
            let msg = "TEST _ A ";
            serial.write(msg.as_bytes()).unwrap();
            x = 0;
            led.toggle();
        }

        if !usb_device.poll(&mut [&mut serial]) {
            continue;
        }
        let mut buf = [0u8; 128];
        match serial.read(&mut buf) {
            Ok(count) if count > 0 => {
                led.set_low(); // Turn on

                // Echo back in upper case
                for c in buf[0..count].iter_mut() {
                    if 0x61 <= *c && *c <= 0x7a {
                        *c &= !0x20;
                    }
                }

                let mut write_offset = 0;
                while write_offset < count {
                    match serial.write(&buf[write_offset..count]) {
                        Ok(len) if len > 0 => {
                            write_offset += len;
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }

        led.set_high(); // Turn off

        /*
        if !nrf_chip.data_available().unwrap() {
            // No data availble, wait 1ms, then check again
            delay.delay_ms(1u16);
            continue;
        }
        led.toggle();
        // Now there is some data availble to read

        let base_msg = b"This is a test message      "; 

        {
            let buffer = block.as_bytes_mut();
            let len = nrf_chip.read(buffer).unwrap();
            assert_eq!(nrf24_rs::MAX_PAYLOAD_SIZE, len as u8);
        }
        let index = block.tag().get_index();
        //block.do_cipher(&cipher);
        //assert_eq!(&block.as_bytes()[4..], base_msg);
        for i in 0..7 {
            assert_eq!(block.data()[i], i as u32);
        }

        delay.delay_ms(200u16);
        led.toggle();
        */
    }
}
