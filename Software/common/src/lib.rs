#![deny(unsafe_op_in_unsafe_fn)]
#![cfg_attr(not(feature = "std"), no_std)]

mod key;
pub use key::*;

mod algorithm;
use algorithm::*;

fn main() {
    use std::io::{Read, Write};

    let mut block_bytes = [0u8; 32];
    std::io::stdin().read_exact(&mut block_bytes).unwrap();
    let mut key_bytes = [0u8; 1024];
    std::io::stdin().read_exact(&mut key_bytes).unwrap();
    let key = Key::new(key_bytes);
    let mut index_key = [0u8; 4];
    std::io::stdin().read_exact(&mut index_key).unwrap();
    let index_key = u32::from_ne_bytes(index_key);

    let mut block = CipherBlock::new(block_bytes);
    let cipher = MainCipher::new(&key, index_key);
    cipher.cipher_block(0, &mut block).unwrap();
    std::io::stdout().write_all(&block).unwrap();
}
