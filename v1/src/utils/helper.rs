use guid_create::GUID;
use murmur3::murmur3_x86_128;
use std::io::Cursor;

pub fn get_guid_value() -> u64 {
    let _u64_max_value: u64 = 18446744073709551615;
    let guid = GUID::rand();
    let guid = guid.to_string();
    let guid_str = guid.replace("-", "");
    let hash_result = murmur3_x86_128(&mut Cursor::new(guid_str), 0).unwrap();
    let uuid = (hash_result >> 65) as u64;

    uuid
}
