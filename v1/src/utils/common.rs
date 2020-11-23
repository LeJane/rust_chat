use anyhow::Result;
use byteorder::{LittleEndian, WriteBytesExt};
use std::io::Cursor;
use crate::utils::binary_helper::*;

pub trait BinaryEncode {
    fn encode(&self) -> Result<Vec<u8>>;
}

//empty str->"".
impl BinaryEncode for &str {
    fn encode(&self) -> Result<Vec<u8>> {
        let mut encoded = Vec::new();

        let length = self.len();
        encoded.write_i16::<LittleEndian>(length as i16)?;

        if length > 0 {
            encoded.extend_from_slice(&*self.as_bytes());
        }

        Ok(encoded)
    }
}


pub trait BinaryDecode<'a> {
    fn decode(cursor: &mut Cursor<&'a [u8]>, bytes: &'a [u8]) -> Result<Self>
        where
            Self: std::marker::Sized;
}

impl<'a, T: BinaryDecode<'a>> BinaryDecode<'a> for Vec<T> {
    fn decode(cursor: &mut Cursor<&'a [u8]>, bytes: &'a [u8]) -> Result<Self> {
        let array_length = binary_read_i16(cursor)?;

        let mut datas = Vec::new();

        if array_length > 0 {
            let mut len = 0;
            loop {
                len += 2;

                if len >= array_length {
                    break;
                }

                let item_length = binary_read_i16(cursor)?;

                let data = T::decode(cursor, &bytes)?;

                datas.push(data);

                len += item_length;
            }
        }

        Ok(datas)
    }
}

pub fn deserialize_binary<'a, T: BinaryDecode<'a> + BinaryEncode>(
    cursor: &mut Cursor<&'a [u8]>,
    bytes: &'a [u8],
) -> Result<T> {
    T::decode(cursor, bytes)
}

impl BinaryEncode for Vec<u8> {
    fn encode(&self) -> Result<Vec<u8>> {
        let encoded_length = self.len();

        let mut data = Vec::with_capacity(encoded_length + 4);

        data.write_i16::<LittleEndian>(encoded_length as i16)?;

        data.extend(self);

        Ok(data)
    }
}

impl<T: BinaryEncode> BinaryEncode for Vec<T> {
    fn encode(&self) -> Result<Vec<u8>> {
        let mut encoded = Vec::new();

        self.into_iter()
            .map(|e| {
                if let Ok(v) = e.encode() {
                    if !v.is_empty() {
                        encoded.extend_from_slice(&v)
                    }
                }
            })
            .count();

        let mut res = Vec::with_capacity(encoded.len() + 4);

        res.write_i16::<LittleEndian>(encoded.len() as i16)?;

        let encoded_length = encoded.len() as i32;

        if encoded_length > 0 {
            res.extend_from_slice(&encoded);
        }

        Ok(res)
    }
}
