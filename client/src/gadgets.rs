// pub fn simple_encode(input: &str) -> String {
//     let encode_data = general_purpose::STANDARD.encode(input);
//     let hex_encode_data = hex::encode(encode_data);
//     hex_encode_data
// }

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
