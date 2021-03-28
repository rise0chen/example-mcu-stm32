use crate::common::hal;
use core::fmt::Write;
use hal::gpio::{
    gpioa::{PA10, PA9},
    *,
};
use hal::pac;
use hal::serial::{Serial, Tx};
use log::{error, LevelFilter, Metadata, Record};
use spin::Mutex;
use spin::Once;

static LOGGER: Once<Logger> = Once::new();
pub struct Logger {
    uart: Mutex<Tx<pac::USART1>>,
}
unsafe impl Send for Logger {}
unsafe impl Sync for Logger {}
impl Logger {
    pub fn start(
        uart: Serial<pac::USART1, (PA9<Alternate<PushPull>>, PA10<Input<Floating>>)>,
        filter: LevelFilter,
    ) {
        let tx = uart.split().0;
        let logger = LOGGER.call_once(|| Logger {
            uart: Mutex::new(tx),
        });

        if let Ok(_) = log::set_logger(logger) {
            log::set_max_level(filter);
        };
    }
}
impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let target = if !record.target().is_empty() {
                record.target()
            } else {
                record.module_path().unwrap_or_default()
            };
            writeln!(
                self.uart.lock(),
                "{} [{}] {}\r\n",
                record.level(),
                target,
                record.args()
            )
            .expect("Error occurred while trying to write uart.");
        }
    }

    fn flush(&self) {}
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    error!("{}", info);
    loop {}
}
