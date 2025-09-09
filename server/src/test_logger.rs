#![cfg(test)]

use std::sync::{Mutex, Once};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::fmt::MakeWriter;

pub struct TestLogger {
    pub messages: Mutex<Vec<String>>,
}

pub static LOGGER: TestLogger = TestLogger {
    messages: Mutex::new(Vec::new()),
};

pub static INIT: Once = Once::new();

pub fn init(level: LevelFilter) {
    struct Writer;

    impl std::io::Write for Writer {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            LOGGER
                .messages
                .lock()
                .unwrap()
                .push(String::from_utf8_lossy(buf).trim().to_string());
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    struct Factory;
    impl<'a> MakeWriter<'a> for Factory {
        type Writer = Writer;
        fn make_writer(&'a self) -> Self::Writer {
            Writer
        }
    }

    let subscriber = tracing_subscriber::fmt()
        .with_writer(Factory)
        .with_max_level(level)
        .without_time()
        .finish();

    let _ = tracing::subscriber::set_global_default(subscriber);
}

