use std::mem;

use serde::{Deserialize, Serialize};
use tiny_keccak::{Hasher, Sha3};

pub type Digest = [u8; 32];

#[derive(Debug)]
pub struct MerkleTree<T> {
    levels: Vec<Vec<Digest>>,
    values: Vec<T>,
    root_hash: Digest,
}

impl<T: AsRef<[u8]> + Clone> MerkleTree<T> {
    pub fn from_vec(values: Vec<T>) -> Self {
        let mut levels = Vec::new();
        let mut cur_lvl: Vec<Digest> = values.iter().map(hash).collect();
        while cur_lvl.len() > 1 {
            let next_lvl = cur_lvl.chunks(2).map(hash_chunk).collect();
            levels.push(mem::replace(&mut cur_lvl, next_lvl));
        }
        let root_hash = cur_lvl[0];
        MerkleTree {
            levels,
            values,
            root_hash,
        }
    }

    pub fn proof(&self, index: usize) -> Option<Proof<T>> {
        let value = self.values.get(index)?.clone();
        let mut lvl_i = index;
        let mut digests = Vec::new();
        for level in &self.levels {
            if let Some(digest) = level.get(lvl_i ^ 1) {
                digests.push(*digest);
            }
            lvl_i /= 2;
        }
        Some(Proof {
            index,
            digests,
            value,
            root_hash: self.root_hash,
        })
    }

    pub fn root_hash(&self) -> &Digest {
        &self.root_hash
    }

    pub fn values(&self) -> &[T] {
        &self.values
    }

    pub fn into_values(self) -> Vec<T> {
        self.values
    }

    pub fn get_value_num(self) -> usize{
        self.values.len()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq,Eq,Hash)]
pub struct Proof<T> {
    value: T,
    index: usize,
    digests: Vec<Digest>,
    root_hash: Digest,
}

impl<T: AsRef<[u8]>> Proof<T> {

    pub fn validate(&self, n: usize) -> bool {
        let mut digest = hash(&self.value);
        let mut lvl_i = self.index;
        let mut lvl_n = n;
        let mut digest_itr = self.digests.iter();
        while lvl_n > 1 {
            if lvl_i ^ 1 < lvl_n {
                digest = match digest_itr.next() {
                    None => return false, // Not enough levels in the proof.
                    Some(sibling) if lvl_i & 1 == 1 => hash_pair(&sibling, &digest),
                    Some(sibling) => hash_pair(&digest, &sibling),
                };
            }
            lvl_i /= 2; 
            lvl_n = (lvl_n + 1) / 2; 
        }
        if digest_itr.next().is_some() {
            return false; 
        }
        digest == self.root_hash
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn root_hash(&self) -> &Digest {
        &self.root_hash
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn into_value(self) -> T {
        self.value
    }
}


fn hash_chunk(chunk: &[Digest]) -> Digest {
    if chunk.len() == 1 {
        chunk[0]
    } else {
        hash_pair(&chunk[0], &chunk[1])
    }
}

fn hash_pair<T0: AsRef<[u8]>, T1: AsRef<[u8]>>(v0: &T0, v1: &T1) -> Digest {
    let bytes: Vec<u8> = v0.as_ref().iter().chain(v1.as_ref()).cloned().collect();
    hash(&bytes)
}

fn hash<T: AsRef<[u8]>>(value: T) -> Digest {
    let mut sha3 = Sha3::v256();
    sha3.update(value.as_ref());

    let mut out = [0u8; 32];
    sha3.finalize(&mut out);
    out
}
