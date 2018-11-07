extern crate chrono;

use std::sync::Arc;
use std::sync::Mutex;
use chrono::prelude::*;

#[derive(Default, Debug)]
struct Snowflake {
    twepoch: i64,

    datacenter_bits: u8,
    worker_bits: u8,
    sequence_bits: u8,

    datacenter_id: i64,
    worker_id: i64,
    sequence_id: i64,

    datacenter_id_shift: u8,
    worker_id_shift: u8,
    timestamp_shift: u8,
    sequence_mask: i64,

    timestamp: i64,
    last_timestamp: Arc<Mutex<i64>>,
}

impl Snowflake {
    fn new(worker_id: i64, datacenter_id: i64) -> Snowflake {
        Snowflake {
            twepoch: 1541488102,

            datacenter_bits: 5,
            worker_bits: 5,
            sequence_bits: 12,

            datacenter_id: datacenter_id,
            worker_id: worker_id,
            sequence_id: 0i64,

            datacenter_id_shift: 17,
            worker_id_shift: 12,
            timestamp_shift: 22,
            sequence_mask: -1i64 ^ (-1i64 << 12),

            timestamp: 0,
            last_timestamp: Arc::new(Mutex::new(0)),
        }
    }

    pub fn generate_id(&mut self) -> Result<i64, String> {
        let mut last_timestamp = self.last_timestamp.lock().unwrap();

        let mut timestamp = Snowflake::ukg_current_time();
        if timestamp < *last_timestamp {
            return Err(format!("Clock moved backwards.  Refusing to generate id for {} milliseconds", *last_timestamp));
        }

        if timestamp == *last_timestamp {
            self.sequence_id = (self.sequence_id + 1) & self.sequence_mask;
            if self.sequence_id == 0 {
                if timestamp == *last_timestamp {
                    timestamp = self.ukg_next_millis(*last_timestamp);
                }
            }
        } else {
            self.sequence_id = 0i64;
        }

        *last_timestamp = timestamp;

        Ok((timestamp - self.twepoch << self.timestamp_shift)
            | (self.datacenter_id << self.datacenter_id_shift)
            | (self.worker_id << self.worker_id_shift)
            | (self.sequence_id))
    }

    fn ukg_current_time() -> i64 {
        let local: DateTime<Local> = Local::now();

        local.timestamp_millis()
    }

    fn ukg_next_millis(&self, last_timestamp: i64) -> i64 {
        let mut timestamp = Snowflake::ukg_current_time();
        while timestamp <= last_timestamp {
            timestamp = Snowflake::ukg_current_time();
        }
        timestamp
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::thread;
    use std::time::Instant;

    #[test]
    fn loop_test() {
        let mut worker = Snowflake::new(1, 1);
        println!("{:?}", &worker);
        for _ in 1..1000 {
            let t = &mut worker;
            assert!(t.generate_id().is_ok());
        }
    }

    #[test]
    fn multi_thread() {
        let now = Instant::now();
        for i in 1..10 {
            let mut worker = Snowflake::new(i, 1);
            thread::spawn(move || {
                for _ in 1..1000000 {
                    let t = &mut worker;
                    let new_id = t.generate_id().unwrap();
                    let id = t.generate_id().unwrap();
                    assert_ne!(new_id, id);
                }
            });
        }

        let elapsed = now.elapsed();
        println!("{}.{}", elapsed.as_secs(), elapsed.subsec_nanos());
    }
}