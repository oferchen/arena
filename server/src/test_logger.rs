#![cfg(test)]

use log::{Log, Metadata, Record};
use std::sync::{Mutex, Once};

pub struct TestLogger {
    pub messages: Mutex<Vec<String>>,
}

impl Log for TestLogger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            self.messages
                .lock()
                .unwrap()
                .push(record.args().to_string());
        }
    }

    fn flush(&self) {}
}

pub static LOGGER: TestLogger = TestLogger {
    messages: Mutex::new(Vec::new()),
};

pub static INIT: Once = Once::new();
