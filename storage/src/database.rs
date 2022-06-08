use bigint::{U512};
use leveldb::database::Database;
use std::{env, fs};
// use leveldb::iterator::Iterable;
use crate::common::{DBKey};
use leveldb::kv::KV;
use leveldb::options::{Options, ReadOptions, WriteOptions};
pub struct LevelDB(Database<DBKey>);

impl LevelDB {
    pub fn new(path: &str) -> Self {
        let mut dir = env::current_dir().unwrap();
        dir.push(path);

        let path_buf = dir.clone();
        fs::create_dir_all(dir).unwrap();

        let path = path_buf.as_path();
        let mut options = Options::new();
        options.create_if_missing = true;

        let database = match Database::open(path, options) {
            Ok(db) => db,
            Err(e) => {
                panic!("Open database error: {:?}", e)
            }
        };

        LevelDB(database)
    }

    pub fn read(&self, key: &[u8]) -> Option<Vec<u8>> {
        let db_key = DBKey {
            val: U512::from(key),
        };
        let read_opts = ReadOptions::new();
        let res = self.0.get(read_opts, db_key);

        match res {
            Ok(data) => data,
            Err(e) => {
                eprintln!("error: {}", e);
                None
            }
        }
    }

    pub fn write(&mut self, key: &[u8], value: &[u8]) {
        let db_key = DBKey {
            val: U512::from(key),
        };
        let write_opts = WriteOptions::new();
        match self.0.put(write_opts, db_key, &value) {
            Ok(_) => (),
            Err(e) => {
                eprintln!("Write to database error: {:?}", e)
            }
        };
    }
}


#[cfg(test)]
mod database_test {
    use super::LevelDB;

    #[test]
    fn leveldb_works() {
        let mut db = LevelDB::new("./test");
        let key = "wangjitao".as_bytes();
        println!("key len: {}", key.len());

        let value = "wangchenxing".as_bytes();
        db.write(key, value);

        let result = db.read(key).unwrap();
        println!("result: {:?}", result);
    }
}