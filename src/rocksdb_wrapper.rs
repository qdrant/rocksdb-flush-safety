use std::path::Path;
use std::sync::Arc;

use parking_lot::RwLock;
use rocksdb::{ColumnFamily, DBRecoveryMode, Error, LogLevel, Options, WriteOptions, DB};

const DB_CACHE_SIZE: usize = 10 * 1024 * 1024; // 10 mb
const DB_MAX_LOG_SIZE: usize = 1024 * 1024; // 1 mb
const DB_MAX_OPEN_FILES: usize = 256;
const DB_DELETE_OBSOLETE_FILES_PERIOD: u64 = 3 * 60 * 1_000_000; // 3 minutes in microseconds

pub const DB_VECTOR_CF: &str = "vector";
pub const DB_MAPPING_CF: &str = "mapping";
pub const DB_DEFAULT_CF: &str = "default";

/// RocksDB options (both global and for column families)
pub fn db_options() -> Options {
    let mut options: Options = Options::default();
    options.set_write_buffer_size(DB_CACHE_SIZE); // write_buffer_size is enforced per column family.
    options.create_if_missing(true);
    options.set_log_level(LogLevel::Error);
    options.set_recycle_log_file_num(1);
    options.set_keep_log_file_num(1); // must be greater than zero
    options.set_max_log_file_size(DB_MAX_LOG_SIZE);
    options.set_delete_obsolete_files_period_micros(DB_DELETE_OBSOLETE_FILES_PERIOD);
    options.create_missing_column_families(true);
    options.set_max_open_files(DB_MAX_OPEN_FILES as i32);

    // Qdrant relies on it's own WAL for durability
    options.set_wal_recovery_mode(DBRecoveryMode::TolerateCorruptedTailRecords);
    options.set_paranoid_checks(true);

    options
}

pub fn open_db(path: &Path) -> Result<Arc<RwLock<DB>>, rocksdb::Error> {
    let column_families = vec![DB_MAPPING_CF, DB_VECTOR_CF, DB_DEFAULT_CF];
    let options = db_options();
    // Make sure that all column families have the same options
    let column_with_options = column_families
        .into_iter()
        .map(|cf| (cf, options.clone()))
        .collect::<Vec<_>>();
    let db = DB::open_cf_with_opts(&options, path, column_with_options)?;
    Ok(Arc::new(RwLock::new(db)))
}

#[derive(Clone)]
pub struct DatabaseColumnWrapper {
    pub database: Arc<RwLock<DB>>,
    pub column_name: String,
    pub use_wal: bool,
}

impl DatabaseColumnWrapper {
    pub fn new(database: Arc<RwLock<DB>>, column_name: &str, use_wal: bool) -> Self {
        Self {
            database,
            column_name: column_name.to_string(),
            use_wal,
        }
    }

    pub fn put<K, V>(&self, key: K, value: V) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        let db = self.database.read();
        let cf_handle = self.get_column_family(&db)?;
        db.put_cf_opt(cf_handle, key, value, &self.get_write_options())?;
        Ok(())
    }

    pub fn exists<K>(&self, key: K) -> Result<bool, Error>
    where
        K: AsRef<[u8]>,
    {
        let db = self.database.read();
        let cf_handle = self.get_column_family(&db)?;
        db.get_cf(cf_handle, key).map(|v| v.is_some())
    }

    pub fn flush(&self) -> Result<(), Error> {
        let database = self.database.clone();
        let column_name = self.column_name.clone();
        let db = database.read();
        let Some(column_family) = db.cf_handle(&column_name) else {
            // It is possible, that the index was removed during the flush by user or another thread.
            // In this case, non-existing column family is not an error, but an expected behavior.

            // Still we want to log this event, for potential debugging.
            log::warn!(
                "Flush: RocksDB cf_handle error: Cannot find column family {}. Ignoring",
                &column_name
            );
            return Ok(()); // ignore error
        };

        db.flush_cf(column_family)?;
        Ok(())
    }

    pub fn create_column_family_if_not_exists(&self) -> Result<(), Error> {
        let mut db = self.database.write();
        if db.cf_handle(&self.column_name).is_none() {
            db.create_cf(&self.column_name, &db_options())?
        }
        Ok(())
    }

    pub fn recreate_column_family(&self) -> Result<(), Error> {
        self.remove_column_family()?;
        self.create_column_family_if_not_exists()
    }

    pub fn remove_column_family(&self) -> Result<(), Error> {
        let mut db = self.database.write();
        if db.cf_handle(&self.column_name).is_some() {
            db.drop_cf(&self.column_name)?;
        }
        Ok(())
    }

    fn get_write_options(&self) -> WriteOptions {
        let mut write_options = WriteOptions::default();
        write_options.set_sync(false);
        write_options.disable_wal(!self.use_wal);
        write_options
    }

    fn get_column_family<'a>(
        &self,
        db: &'a parking_lot::RwLockReadGuard<'_, DB>,
    ) -> Result<&'a ColumnFamily, Error> {
        Ok(db.cf_handle(&self.column_name).unwrap())
    }
}
