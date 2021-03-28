#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;
mod common;
mod serial;

use common::logger::Logger;
use core::mem::MaybeUninit;
use core::time::Duration;
use cortex_m_rt::entry;
use embedded_hal::digital::v2::OutputPin;
use log::{LevelFilter, *};
use stm32f1xx_hal::{
    gpio::*,
    pac::{self, interrupt, Interrupt},
    prelude::*,
    serial::{Config, Serial},
    timer::{CountDownTimer, Event, Timer},
};
use task_stream::TaskStream;

static mut TIMER: MaybeUninit<CountDownTimer<pac::TIM2>> = MaybeUninit::uninit();

#[interrupt]
fn TIM2() {
    task_stream::tick(100);
    let _err = unsafe { (TIMER.as_mut_ptr().as_mut().unwrap()).wait() };
}

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();
    let _cp = cortex_m::Peripherals::take().unwrap();
    let mut flash = dp.FLASH.constrain();
    let mut rcc = dp.RCC.constrain();
    let clocks = rcc.cfgr.freeze(&mut flash.acr);
    let mut afio = dp.AFIO.constrain(&mut rcc.apb2);
    let mut gpioa = dp.GPIOA.split(&mut rcc.apb2);
    let mut gpioc = dp.GPIOC.split(&mut rcc.apb2);

    common::init_alloc();
    // bsp
    let mut led = gpioc.pc13.into_open_drain_output(&mut gpioc.crh);

    // logger
    let tx = gpioa.pa9.into_alternate_push_pull(&mut gpioa.crh);
    let rx = gpioa.pa10;
    let serial = Serial::usart1(
        dp.USART1,
        (tx, rx),
        &mut afio.mapr,
        Config::default().baudrate(115200.bps()),
        clocks,
        &mut rcc.apb2,
    );
    Logger::start(serial, LevelFilter::Trace);
    info!("start.");

    // set timer
    let mut timer = Timer::tim2(dp.TIM2, &clocks, &mut rcc.apb1).start_count_down(10.hz());
    timer.listen(Event::Update);
    unsafe {
        cortex_m::peripheral::NVIC::unmask(Interrupt::TIM2);
    }
    unsafe { TIMER.as_mut_ptr().write(timer) };

    // USART
    let tx = gpioa.pa2.into_alternate_push_pull(&mut gpioa.crl);
    let rx = gpioa.pa3;
    let serial = Serial::usart2(
        dp.USART2,
        (tx, rx),
        &mut afio.mapr,
        Config::default().baudrate(115200.bps()),
        clocks,
        &mut rcc.apb1,
    );
    serial::start(serial);

    // blinky
    task_stream::spawn(async move {
        let mut i: u32 = 0;
        loop {
            task_stream::sleep(Duration::from_millis(500)).await;
            led.set_high().unwrap();
            task_stream::sleep(Duration::from_millis(500)).await;
            led.set_low().unwrap();
            serial::send(&i.to_be_bytes());
            i = i.wrapping_add(1);
        }
    });

    // async executor
    let stream = TaskStream::stream();
    loop {
        while let Some(task) = stream.get_task() {
            task.run();
        }
        cortex_m::asm::wfi();
    }
}
