use sha2::{Digest, Sha512};

pub trait ShaSnap: AsRef<[u8]> {
    fn snap(&self) -> String {
        let mut hasher = Sha512::new();
        hasher.update(&self);
        let result = hasher.finalize();
        format!("{:x}", result)
    }
}

impl<T: AsRef<[u8]>> ShaSnap for T {}
