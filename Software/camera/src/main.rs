#![no_std]
#![no_main]
#![feature(bench_black_box)]

use cortex_m_rt::entry;
use hal::{
    gpio::Edge,
    pac::{dcmi, DCMI},
    prelude::*,
};
use panic_halt as _;
use stm32f4xx_hal as hal;

#[entry]
fn main() -> ! {
    let device = hal::pac::Peripherals::take().unwrap();
    let core = cortex_m::Peripherals::take().unwrap();

    // Enable the clock for the DCMI and GPIOs
    device.RCC.ahb2enr.write(|w| w.dcmien().set_bit());
    device.RCC.ahb1enr.write(|w| w.gpioien().set_bit());

    //(#) DCMI pins configuration
    //  (++) Connect the involved DCMI pins to AF13 using the following function
    //      GPIO_PinAFConfig(GPIOx, GPIO_PinSourcex, GPIO_AF_DCMI);
    //  (++) Configure these DCMI pins in alternate function mode by calling
    //      the function GPIO_Init();
    //
    // ???

    device.DCMI.cr.write(|w| w);
    // DCMI_InitStruct->DCMI_CaptureMode = DCMI_CaptureMode_Continuous;
    // DCMI_InitStruct->DCMI_SynchroMode = DCMI_SynchroMode_Hardware;
    // DCMI_InitStruct->DCMI_PCKPolarity = DCMI_PCKPolarity_Falling;
    // DCMI_InitStruct->DCMI_VSPolarity = DCMI_VSPolarity_Low;
    // DCMI_InitStruct->DCMI_HSPolarity = DCMI_HSPolarity_Low;
    // DCMI_InitStruct->DCMI_CaptureRate = DCMI_CaptureRate_All_Frame;
    // DCMI_InitStruct->DCMI_ExtendedDataMode = DCMI_ExtendedDataMode_8b;

    // # Safety: We got the bits from a read of the register
    let dcmi = device.DCMI;
    //Looks like no high level wrapper exists for DCMI from HAL.
    //We have to use the auto generated building blocks

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
