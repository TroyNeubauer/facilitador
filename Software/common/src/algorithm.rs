
use core::marker::PhantomData;
use core::ops::Deref;

//use crate::Key;

pub trait Index: core::ops::BitXor<Output = Self> + Sized + Copy {
    fn to_usize(self) -> usize;
}

#[repr(C, align(4))]
pub struct GenericCipherBlock<const N: usize>(pub [u8; N]);

impl<const N: usize> GenericCipherBlock<N> {
    pub fn new(buf: [u8; N]) -> Self {
        Self(buf)
    }
}

pub type CipherBlock = GenericCipherBlock<crate::SUB_KEY_BYTES>;

impl<const N: usize> Deref for GenericCipherBlock<N> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<const N: usize> AsRef<[u8]> for GenericCipherBlock<N> {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

struct GenericCipher<Hash, IndexTy, const KEY_SIZE: usize, const BLOCK_SIZE: usize>
where
    Hash: Fn(IndexTy) -> IndexTy,
    IndexTy: Index,
{
    //hash: Hash,
    hash: Hash,
    key: Key<KEY_SIZE>,
    index_key: IndexTy,
    _index: PhantomData<IndexTy>,
}

impl<'k, Hash, IndexTy, const KEY_SIZE: usize, const BLOCK_SIZE: usize>
    GenericCipher<Hash, IndexTy, KEY_SIZE, BLOCK_SIZE>
where
    Hash: Fn(IndexTy) -> IndexTy,
    IndexTy: Index,
{
    /// Performs encryption or decryption of a single block.
    /// `L` determines many elements the u32 subkey has. Because N is in bytes, `L` should always
    /// be set to N / 4.
    ///
    /// Returns Err on error, and the bytes of `block` are not guaranteed to be encrypted or
    /// decrypted
    /// If Ok(()) is returned, all bytes of `block` have been encrypted/decrypted
    ///
    /// # Panics
    /// This function panics if L is the wrong size.
    /// If L is the correct size for N. This function will never panic, otherwise it will always
    /// panic
    pub fn cipher_block<const L: usize>(
        &self,
        index: IndexTy,
        block: &mut GenericCipherBlock<BLOCK_SIZE>,
    ) -> Result<(), ()> {
        if KEY_SIZE / size_of::<u32>() != L {
            // User choose wrong L for N
            panic!(
                "Wrong L ({}), for N ({}). Expected L to be {}",
                L,
                KEY_SIZE,
                KEY_SIZE / size_of::<u32>()
            );
        }

        // Perform Xor first, so that an attacker doesn't know the inputs to the hash function
        let index = index ^ self.index_key;
        let index = (self.hash)(index);
        let index = index.to_usize();

        let key = self.key.subkey::<L>(index);

        // SAFETY: u8 is safe to transmute to u32. There are no invalid bit patterns
        let (before, buf, after) = unsafe { block.0.align_to_mut::<u32>() };

        // These lengths are guaranteed to be 0 because `CipherBlock` has explicit 4 byte alignment
        debug_assert!(before.is_empty());
        debug_assert!(after.is_empty());
        // Perform Xor encryption
        for i in 0..buf.len() {
            buf[i] ^= key[i];
        }

        Ok(())
    }
}

/*
struct GenericCipher<'k, Hash, const N: usize, IndexSrc, IndexDst, T>
where
    Hash: Fn(IndexSrc) -> IndexDst,
    IndexSrc: byte_slice_cast::AsByteSlice<T>,
    IndexDst: std::ops::BitXor<Output = usize>,
{
*/

fn identity(index: u32) -> u32 {
    index
}

struct MainCipher<Hash, const KEY_SIZE: usize>(GenericCipher<Hash, u32, KEY_SIZE, SUB_KEY_BYTES>)
where
    Hash: Fn(u32) -> u32;

impl<'k, const KEY_SIZE: usize> MainCipher<fn(u32) -> u32, KEY_SIZE> {
    pub fn new(key: [u8; KEY_SIZE], index_key: u32) -> Self {
        Self(GenericCipher {
            hash: identity,
            key: Key(key),
            index_key,
            _index: Default::default(),
        })
    }

    /// Encrypts or decrypts a single block using `key` and `index`.
    /// Because Xor is used, the encryption and decryption operation is the same
    pub fn cipher_block(&self, index: u32, block: &mut GenericCipherBlock<32>) -> Result<(), ()> {
        self.0.cipher_block::<SUB_KEY_ELEMENTS>(index, block)
    }
}

impl Index for u32 {
    fn to_usize(self) -> usize {
        assert!(
            size_of::<usize>() >= size_of::<u32>(),
            "8 and 16 bit targets not supported yet!"
        );
        self.try_into().unwrap()
    }
}

#[cfg(test)]
mod tests2 {
    use super::*;
    use rand::RngCore;

    #[test]
    fn encrypt_and_decrypt() {
        for i in 0..512 {
            let mut rng = rand::thread_rng();

            let mut block_bytes = [0u8; 32];
            rng.fill_bytes(&mut block_bytes);
            let original_block = Clone::clone(&block_bytes);

            let mut key_bytes = [0u8; 32];
            rng.fill_bytes(&mut key_bytes);

            let mut index_key = [0u8; 4];
            rng.fill_bytes(&mut index_key);
            let index_key = u32::from_ne_bytes(index_key);

            let mut block = CipherBlock::new(block_bytes);
            let cipher = MainCipher::new(key_bytes, index_key);
            println!("{:?}", block.as_ref());

            cipher.cipher_block(i, &mut block).unwrap();
            println!("{:?}", block.as_ref());
            cipher.cipher_block(i, &mut block).unwrap();
            assert_eq!(block.as_ref(), original_block.as_ref());

            println!("{:?}", block.as_ref());
        }
    }
}

