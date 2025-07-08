use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose, Engine as _};

pub struct Cipher {
    cipher: Aes256Gcm,
}

impl Cipher {
    pub fn new(key: &[u8; 32]) -> Self {
        let key = Key::<Aes256Gcm>::from_slice(key);
        let cipher = Aes256Gcm::new(key);
        Self { cipher }
    }

    pub fn encrypt(&self, plaintext: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext)
            .map_err(|e| e.to_string())?;

        let mut result = nonce.to_vec();
        result.extend_from_slice(&ciphertext);
        Ok(general_purpose::STANDARD.encode(result))
    }

    pub fn decrypt(&self, ciphertext: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let data = general_purpose::STANDARD.decode(ciphertext)?;
        if data.len() < 12 {
            return Err("Invalid ciphertext length".into());
        }

        let (nonce, ciphertext) = data.split_at(12);
        let nonce = Nonce::from_slice(nonce);
        let plaintext = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| e.to_string())?;
        Ok(plaintext)
    }
}
