use base64::{engine::general_purpose, Engine as _};
use hex;

// pub fn simple_encode(input: &str) -> String {
//     let encode_data = general_purpose::STANDARD.encode(input);
//     let hex_encode_data = hex::encode(encode_data);
//     hex_encode_data
// }

pub fn simple_decode(encoded: &str) -> String {
    let hex_decoded = hex::decode(encoded).unwrap();
    let base64_decoded = general_purpose::STANDARD.decode(hex_decoded).unwrap();
    let plain_string = String::from_utf8(base64_decoded).unwrap();
    plain_string
}

// #[cfg(test)]
// mod tests {
//     use super::simple_encode;
//     use super::*;
//     #[test]
//     fn test_ip_encoding() {
//         let result = simple_encode("http://localhost:8080");
//         println!("Encoded: {}", result);
//     }
// }
