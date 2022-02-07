#![no_std]
#![no_main]
#![feature(bench_black_box)]

use cortex_m_rt::entry;
use hal::{prelude::*, gpio::Edge};
use panic_halt as _;
use stm32f4xx_hal as hal;

#[entry]
fn main() -> ! {
    let device = hal::pac::Peripherals::take().unwrap();
    let core = cortex_m::Peripherals::take().unwrap();

    // Enable the clock for the DCMI and GPIOs
    device.RCC.ahb2enr.write(|w| w.dcmien().set_bit());
    device.RCC.ahb1enr.write(|w| w.gpioien().set_bit());

    let rcc = device.RCC.constrain();
    let clocks = rcc
        .cfgr
        .use_hse(16.mhz())
        .require_pll48clk()
        .sysclk(168.mhz())
        .hclk(168.mhz())
        .pclk1(42.mhz())
        .pclk2(82.mhz())
        .freeze();

    let mut syscfg = device.SYSCFG.constrain();

    // Initialize the different pins
    let gpioc = device.GPIOC.split();
    let gpiod = device.GPIOD.split();
    let gpiob = device.GPIOB.split(); 

    let mut delay = hal::delay::Delay::new(core.SYST, &clocks);

    //Configure camera with I2C
    
    let dcmi = device.DCMI;
    
    loop {}
}
