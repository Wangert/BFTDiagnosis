use bigint::{U256, U512};
use db_key::Key;

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Copy, Clone)]
pub struct BlockKey {
    pub val: U256,
}

impl Key for BlockKey {
    fn from_u8(key: &[u8]) -> Self {
        use std::mem::transmute;

        assert!(key.len() == 32);
        let mut result: [u8; 32] = [0; 32];

        for (i, val) in key.iter().enumerate() {
            result[i] = *val;
        }

        unsafe { transmute::<[u8; 32], Self>(result) }
    }

    fn as_slice<T, F: Fn(&[u8]) -> T>(&self, f: F) -> T {
        use std::mem::transmute;

        let val = unsafe { transmute::<_, &[u8; 32]>(self) };
        f(val)
    }
}

pub struct DBKey {
    pub val: U512,
}

impl Key for DBKey {
    fn from_u8(key: &[u8]) -> Self {
        use std::mem::transmute;

        assert!(key.len() == 64);
        let mut result: [u8; 64] = [0; 64];

        for (i, val) in key.iter().enumerate() {
            result[i] = *val;
        }

        unsafe { transmute::<[u8; 64], Self>(result) }
    }

    fn as_slice<T, F: Fn(&[u8]) -> T>(&self, f: F) -> T {
        use std::mem::transmute;

        let val = unsafe { transmute::<_, &[u8; 64]>(self) };
        f(val)
    }
}
