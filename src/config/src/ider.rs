// Copyright 2024 Zinc Labs Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use std::{
    hint::spin_loop,
    time::{SystemTime, UNIX_EPOCH},
};

use once_cell::sync::Lazy;
use parking_lot::Mutex;
use svix_ksuid::{Ksuid, KsuidLike};

static IDER: Lazy<Mutex<SnowflakeIdGenerator>> = Lazy::new(|| {
    let machine_id = unsafe { super::cluster::LOCAL_NODE_ID };
    log::info!("init ider with machine_id: {}", machine_id);
    Mutex::new(SnowflakeIdGenerator::new(machine_id))
});

pub fn init() {
    _ = generate();
}

/// Generate a distributed unique id with snowflake.
pub fn generate() -> String {
    IDER.lock().generate().to_string()
}

/// Generate a unique id like uuid.
pub fn uuid() -> String {
    Ksuid::new(None, None).to_string()
}

/// The `SnowflakeIdGenerator` type is snowflake algorithm wrapper.
#[derive(Copy, Clone, Debug)]
pub struct SnowflakeIdGenerator {
    /// epoch used by the snowflake algorithm.
    epoch: SystemTime,

    /// last_time_millis, last time generate id is used times millis.
    last_time_millis: i64,

    /// machine_id, is use to supplement id machine or sectionalization attribute.
    pub machine_id: i32,

    /// auto-increment record.
    idx: u16,
}

/// The `SnowflakeIdBucket` type is snowflake-id-bucket it easy to get id also have a id buffer.
#[derive(Clone, Debug)]
pub struct SnowflakeIdBucket {
    /// Hidden the `SnowflakeIdGenerator` in bucket .
    snowflake_id_generator: SnowflakeIdGenerator,

    /// The bucket buffer;
    bucket: Vec<i64>,
}

impl SnowflakeIdGenerator {
    /// Constructs a new `SnowflakeIdGenerator` using the UNIX epoch.
    /// Please make sure that machine_id is small than 1024(2^10);
    pub fn new(machine_id: i32) -> SnowflakeIdGenerator {
        let epoch = UNIX_EPOCH;
        let last_time_millis = get_time_millis(epoch);
        SnowflakeIdGenerator {
            epoch,
            last_time_millis,
            machine_id,
            idx: 0,
        }
    }

    /// The real_time_generate keep id generate time is eq call method time.
    pub fn real_time_generate(&mut self) -> i64 {
        self.idx = (self.idx + 1) % 4096;

        let mut now_millis = get_time_millis(self.epoch);

        // supplement code for 'clock is moving backwards situation'.

        // If the milliseconds of the current clock are equal to
        // the number of milliseconds of the most recently generated id,
        // then check if enough 4096 are generated,
        // if enough then busy wait until the next millisecond.
        if now_millis == self.last_time_millis {
            if self.idx == 0 {
                now_millis = biding_time_conditions(self.last_time_millis, self.epoch);
                self.last_time_millis = now_millis;
            }
        } else {
            self.last_time_millis = now_millis;
            self.idx = 0;
        }

        // last_time_millis is 64 bits, left shift 22 bit, store 42 bits, machine_id left shift 12
        // bits, idx complementing bits.
        self.last_time_millis << 22 | ((self.machine_id << 12) as i64) | (self.idx as i64)
    }

    /// The basic guarantee time punctuality.
    ///
    /// Basic guarantee time punctuality.
    /// sometimes one millis can't use up 4096 ID, the property of the ID isn't real-time.
    /// But setting time after every 4096 calls.
    pub fn generate(&mut self) -> i64 {
        self.idx = (self.idx + 1) % 4096;

        // Maintenance `last_time_millis` for every 4096 ids generated.
        if self.idx == 0 {
            let mut now_millis = get_time_millis(self.epoch);

            if now_millis == self.last_time_millis {
                now_millis = biding_time_conditions(self.last_time_millis, self.epoch);
                if now_millis == self.last_time_millis {
                    panic!(
                        "Clock is moving backwards.  Rejecting requests until {}.",
                        now_millis
                    );
                }
            }

            self.last_time_millis = now_millis;
        }

        // last_time_millis is 64 bits, left shift 22 bit, store 42 bits, machine_id left shift 12
        // bits, idx complementing bits.
        self.last_time_millis << 22 | ((self.machine_id << 12) as i64) | (self.idx as i64)
    }

    /// The lazy generate.
    ///
    /// Lazy generate.
    /// Just start time record last_time_millis it consume every millis ID.
    /// Maybe faster than standing time.
    pub fn lazy_generate(&mut self) -> i64 {
        self.idx = (self.idx + 1) % 4096;

        if self.idx == 0 {
            self.last_time_millis += 1;
        }

        // last_time_millis is 64 bits, left shift 22 bit, store 42 bits, machine_id left shift 12
        // bits, idx complementing bits.
        self.last_time_millis << 22 | ((self.machine_id << 12) as i64) | (self.idx as i64)
    }
}

impl SnowflakeIdBucket {
    /// Constructs a new `SnowflakeIdBucket` using the UNIX epoch.
    /// Please make sure that machine_id is small than 1024(2^10);
    pub fn new(machine_id: i32) -> Self {
        let snowflake_id_generator = SnowflakeIdGenerator::new(machine_id);
        let bucket = Vec::new();

        SnowflakeIdBucket {
            snowflake_id_generator,
            bucket,
        }
    }

    pub fn get_id(&mut self) -> i64 {
        if self.bucket.is_empty() {
            self.generate_ids();
            self.bucket.reverse();
        }
        self.bucket.pop().unwrap()
    }

    fn generate_ids(&mut self) {
        for _ in 0..4091 {
            self.bucket
                .push(self.snowflake_id_generator.lazy_generate());
        }
    }
}

#[inline(always)]
/// Get the latest milliseconds of the clock.
pub fn get_time_millis(epoch: SystemTime) -> i64 {
    SystemTime::now()
        .duration_since(epoch)
        .expect("Time went backward")
        .as_millis() as i64
}

#[inline(always)]
// Constantly refreshing the latest milliseconds by busy waiting.
fn biding_time_conditions(last_time_millis: i64, epoch: SystemTime) -> i64 {
    let mut latest_time_millis: i64;
    loop {
        latest_time_millis = get_time_millis(epoch);
        if latest_time_millis > last_time_millis {
            return latest_time_millis;
        }
        spin_loop();
    }
}
