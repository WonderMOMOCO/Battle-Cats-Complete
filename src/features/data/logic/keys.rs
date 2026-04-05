use aes::Aes128;
use aes::cipher::{BlockDecryptMut, KeyIvInit, KeyInit};
use block_padding::Pkcs7;
use md5;

type Aes128Cbc = cbc::Decryptor<Aes128>;
type Aes128Ecb = ecb::Decryptor<Aes128>;

pub fn get_md5_key(text: &str) -> [u8; 16] {
    let digest = md5::compute(text.as_bytes());
    let mut key = [0u8; 16];
    let hex_string = hex::encode(&digest.0);
    key.copy_from_slice(&hex_string.as_bytes()[0..16]);
    key
}

fn decrypt_cbc_with_key(data: &[u8], key: &[u8; 16], iv: &[u8; 16]) -> Result<Vec<u8>, String> {
    let decryptor = Aes128Cbc::new(key.into(), iv.into());
    let mut buffer = data.to_vec();
    let len = decryptor
        .decrypt_padded_mut::<Pkcs7>(&mut buffer)
        .map_err(|_| "Padding Error".to_string())?
        .len();
    buffer.truncate(len);
    Ok(buffer)
}

pub fn decrypt_ecb_with_key(data: &[u8], key: &[u8; 16]) -> Result<Vec<u8>, String> {
    let decryptor = Aes128Ecb::new(key.into());
    let mut buffer = data.to_vec();
    let len = decryptor
        .decrypt_padded_mut::<Pkcs7>(&mut buffer)
        .map_err(|_| "Padding Error".to_string())?
        .len();
    buffer.truncate(len);
    Ok(buffer)
}

fn is_content_valid(data: &[u8], filename: &str) -> bool {
    let lower_name = filename.to_lowercase();
    
    if lower_name.ends_with(".png") {
        if data.len() < 4 { return false; }
        return data.starts_with(&[0x89, 0x50, 0x4E, 0x47]);
    }

    if lower_name.ends_with(".csv") 
        || lower_name.ends_with(".list") 
        || lower_name.ends_with(".json")
        || lower_name.ends_with(".maanim")   
        || lower_name.ends_with(".mamodel")  
        || lower_name.ends_with(".imgcut")   
    {
        return std::str::from_utf8(data).is_ok();
    }

    true
}

pub fn decrypt_pack_chunk(data: &[u8], internal_filename: &str) -> Result<(Vec<u8>, String), String> {
    let keys = [
        ("d754868de89d717fa9e7b06da45ae9e3", "40b2131a9f388ad4e5002a98118f6128", "JP"),
        ("0ad39e4aeaf55aa717feb1825edef521", "d1d7e708091941d90cdf8aa5f30bb0c2", "EN"),
        ("313d9858a7fb939def1d7d859629087d", "0e3743eb53bf5944d1ae7e10c2e54bdf", "TW"),
        ("bea585eb993216ef4dcb88b625c3df98", "9b13c2121d39f1353a125fed98696649", "KR"),
    ];

    for (k_hex, iv_hex, region) in keys.iter() {
        let key_bytes = hex::decode(k_hex).unwrap();
        let iv_bytes = hex::decode(iv_hex).unwrap();
        let key_arr: [u8; 16] = key_bytes.try_into().unwrap();
        let iv_arr: [u8; 16] = iv_bytes.try_into().unwrap();

        if let Ok(result) = decrypt_cbc_with_key(data, &key_arr, &iv_arr) {
            if is_content_valid(&result, internal_filename) {
                return Ok((result, region.to_string()));
            }
        }
    }

    let server_key = get_md5_key("battlecats");
    if let Ok(result) = decrypt_ecb_with_key(data, &server_key) {
        if is_content_valid(&result, internal_filename) {
            return Ok((result, "Server".to_string()));
        }
    }

    Ok((data.to_vec(), "None".to_string()))
}