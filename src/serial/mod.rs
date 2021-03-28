use crate::common::hal;
use core::time::Duration;
use eds::consts::FRAME_LEN;
use eds::{Reader, Writer};
use fixed_queue::spsc::{Receiver, Sender, Spsc};
use fixed_queue::Vec;
use hal::gpio::{
    gpioa::{PA2, PA3},
    *,
};
use log::*;
use hal::pac::{self, interrupt, Interrupt};
use hal::prelude::_embedded_hal_serial_Write;
use hal::prelude::*;
use hal::serial::{Rx, Serial, Tx};
use nb::block;
use spin::Mutex;
use spin::{Lazy, Once};

static TX: Once<Mutex<Tx<pac::USART2>>> = Once::new();
static RX: Once<Mutex<Rx<pac::USART2>>> = Once::new();

const SPSC_LEN: usize = FRAME_LEN;
static SPSC: Spsc<u8, SPSC_LEN> = Spsc::new();
static SENDER: Lazy<Sender<'static, u8, SPSC_LEN>> = Lazy::new(|| SPSC.take_sender().unwrap());
static RECVER: Lazy<Receiver<'static, u8, SPSC_LEN>> = Lazy::new(|| SPSC.take_recver().unwrap());
static FRAMER: Lazy<Mutex<Writer>> = Lazy::new(|| Mutex::new(Writer::new(4)));

pub fn send(buf: &[u8]) {
    let mut tx = TX.get().unwrap().lock();
    FRAMER.lock().get_frame(&buf).iter().for_each(|res| {
        block!(tx.write(*res)).ok();
    });
}

#[interrupt]
fn USART2() {
    let mut rx = RX.get().unwrap().lock();
    while let Ok(res) = rx.read() {
        let _err = SENDER.send(res);
    }
}

pub fn start(socket: Serial<pac::USART2, (PA2<Alternate<PushPull>>, PA3<Input<Floating>>)>) {
    let (tx, mut rx) = socket.split();
    rx.listen();
    unsafe {
        cortex_m::peripheral::NVIC::unmask(Interrupt::USART2);
    }
    task_stream::spawn(async move {
        let mut reader = Reader::new();
        let mut vec: Vec<u8, FRAME_LEN> = Vec::new();
        loop {
            if let Ok(res) = RECVER.try_recv() {
                vec.push(res);
            } else {
                task_stream::sleep(Duration::from_millis(100)).await;
            }
            while let Ok(res) = RECVER.try_recv() {
                vec.push(res);
            }
            if vec.len() == 0 {
                continue;
            }
            reader.recv(&vec);
            vec.clear();
            if reader.is_ready() {
                info!("recv: {:02X?}", reader.get_load());
            }
            while !reader.is_finish() {
                reader.recv(&[]);
                if reader.is_ready() {
                    info!("recv: {:02X?}", reader.get_load());
                }
            }
        }
    });
    let _err = TX.call_once(|| Mutex::new(tx));
    let _err = RX.call_once(|| Mutex::new(rx));
}
