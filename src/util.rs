use jsonwebtoken::{DecodingKey, EncodingKey};

pub fn parse_encoding_key(rsa_pem: String) -> EncodingKey {
    let rsa_pem = rsa_pem.replace('_', "\n");
    EncodingKey::from_rsa_pem(rsa_pem.as_bytes()).unwrap()
}

pub fn parse_decoding_key(rsa_pem: String) -> DecodingKey {
    let rsa_pem = rsa_pem.replace('_', "\n");
    DecodingKey::from_rsa_pem(rsa_pem.as_bytes()).unwrap()
}
