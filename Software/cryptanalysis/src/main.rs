use rand::{RngCore, Rng};

fn main() {
    let mut rng = rand::rngs::OsRng;
    //Assert cryptographically secure
    let _: &dyn rand::CryptoRng = &rng;

    let mut plaintext = [0u8; 32];
    let mut key = [0u8; 1024];
    rng.fill_bytes(&mut plaintext);
    rng.fill_bytes(&mut key);

    let cipher = common::a::Cipher::new(key, rng.gen());
    let mut block = common::a::Block::new(plaintext);
    cipher.cipher_block(0, &mut block).unwrap();
    println!("{:b}", V(block.0.to_vec()));
}

fn run_tests(buf: &[u8], msg: impl AsRef<str>) {
    use rand_distr::{StudentT, Distribution, Normal, Exp};
    use statest::ks::*;
    
    let t = StudentT::new(1.0).unwrap();
    let t_vec = (0..1000).map(|_| t.sample(&mut rand::thread_rng()))
                         .collect::<Vec<f64>>();

    let tdist = StudentT::new(1.0f64).unwrap();
    let ndist = Normal::new(0.0f64, 1.0f64).unwrap();
    let edist = Exp::new(1.0f64).unwrap();
    println!("StudentT? {}", t_vec.ks1(&tdist, 0.05)); // true
    println!("Normal? {}", t_vec.ks1(&ndist, 0.05)); // false
    println!("Exponential? {}", t_vec.ks1(&edist, 0.05)); // false
}

struct V(Vec<u8>);

// custom output
impl std::fmt::Binary for V {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // extract the value using tuple idexing
        // and create reference to 'vec'
        let vec = &self.0;

        // @count -> the index of the value,
        // @n     -> the value
        for (count, n) in vec.iter().enumerate() { 
            if count != 0 { write!(f, " ")?; }

            write!(f, "{:010b}", n)?;
        }

        Ok(())
    }
}
