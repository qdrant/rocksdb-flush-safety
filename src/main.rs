use crate::args::Args;
use crate::rocksdb_wrapper::{DB_MAPPING_CF, DB_VECTOR_CF};
use clap::Parser;
use rand::Rng;
use std::process::{Command, Stdio};

mod args;
mod rocksdb_wrapper;

const LOOP_COUNT: usize = 1_000_000;

const TEST_KEY: &str = "test_key";

pub const CRASH_PROBABILITY: f64 = 0.00001;

fn main() {
    let args = Args::parse();

    let flush_interval_ms = args.flush_interval_ms;
    let flush_interval = std::time::Duration::from_millis(flush_interval_ms as u64);

    let data_dir = std::path::Path::new(&args.storage_dir);
    let db = rocksdb_wrapper::open_db(data_dir).unwrap();

    let use_wal = args.wal_enabled;

    let mapping_cf =
        rocksdb_wrapper::DatabaseColumnWrapper::new(db.clone(), DB_MAPPING_CF, use_wal);
    mapping_cf.create_column_family_if_not_exists().unwrap();

    let vector_cf = rocksdb_wrapper::DatabaseColumnWrapper::new(db.clone(), DB_VECTOR_CF, use_wal);
    vector_cf.create_column_family_if_not_exists().unwrap();

    // check if data is consistent after flushing/crash
    for i in 0..LOOP_COUNT {
        let key = make_key(i);
        let mapping_exists = mapping_cf.exists(key.clone()).unwrap();
        if !mapping_exists {
            // nothing to check further
            break;
        }

        // if mapping exists then vector MUST exist given flushing order
        let vector_exists = vector_cf.exists(key.clone()).unwrap();
        if !vector_exists {
            log::error!("Bingo!");
            panic!(
                "Inconsistent flushing - mapping exists but no vector found for key: {}",
                key
            );
        }
    }

    // drop data to not run out of disk space
    mapping_cf.recreate_column_family().unwrap();
    mapping_cf.flush().unwrap();
    vector_cf.recreate_column_family().unwrap();
    vector_cf.flush().unwrap();

    let mut rng = rand::thread_rng();
    let mut start = std::time::Instant::now();
    for i in 0..LOOP_COUNT {
        let key = make_key(i);
        let value = make_value(i);

        // write value in both column families without intermediate flush
        vector_cf.put(key.clone(), value.clone()).unwrap();
        mapping_cf.put(key, value).unwrap();

        // crash before/after flushes sometimes
        if rng.gen_bool(CRASH_PROBABILITY) {
            system("pkill -9 -f rocksdb-flush-safety");
        }

        // flush data every FLUSH_INTERVAL_MS
        if start.elapsed() > flush_interval {
            // reset start instant
            start = std::time::Instant::now();

            // flush vector first
            vector_cf.flush().unwrap();

            // crash between flushes sometimes
            if rng.gen_bool(CRASH_PROBABILITY) {
                system("pkill -9 -f rocksdb-flush-safety");
            }

            // then mapping, therefore if a mapping exists then a vector MUST exist as well :)
            mapping_cf.flush().unwrap();
        }
    }
}

fn make_key(i: usize) -> String {
    format!("{}-{}", TEST_KEY, i)
}

fn make_value(i: usize) -> String {
    i.to_string()
}

fn system(cmd: impl AsRef<str>) {
    Command::new("bash")
        .args(["-c", cmd.as_ref()])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .expect("failed to execute system command");
}
