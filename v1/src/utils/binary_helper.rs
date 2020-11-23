use anyhow::Result;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::Cursor;
use std::str::from_utf8;

pub fn binary_read_i64(cursor: &mut Cursor<&[u8]>) -> Result<i64> {
    let v = cursor.read_i64::<LittleEndian>()?;
    Ok(v)
}

pub fn binary_read_i32(cursor: &mut Cursor<&[u8]>) -> Result<i32> {
    let v = cursor.read_i32::<LittleEndian>()?;
    Ok(v)
}

pub fn binary_read_i16(cursor: &mut Cursor<&[u8]>) -> Result<i16> {
    let v = cursor.read_i16::<LittleEndian>()?;
    Ok(v)
}

pub fn binary_read_i8(cursor: &mut Cursor<&[u8]>) -> Result<i8> {
    let v = cursor.read_i8()?;
    Ok(v)
}

pub fn binary_read_f32(cursor: &mut Cursor<&[u8]>) -> Result<f32> {
    let v = cursor.read_f32::<LittleEndian>()?;
    Ok(v)
}

pub fn binary_read_string(cursor: &mut Cursor<&[u8]>, bytes: &[u8]) -> Result<String> {
    let length = binary_read_i16(cursor)?;
    let length = cursor.position() + length as u64;
    let data = &bytes[cursor.position() as usize..length as usize];
    cursor.set_position(length);

    let v = from_utf8(data)?.to_string();
    Ok(v)
}

pub fn binary_read_msg(cursor: &mut Cursor<&[u8]>, bytes: &[u8]) {
    let msg_length = binary_read_i16(cursor).expect("msg length error");
    let msg_length = cursor.position() + msg_length as u64;
    let _msg = &bytes[cursor.position() as usize..msg_length as usize];
    cursor.set_position(msg_length);
}

pub fn binary_write_i64(encoded: &mut Vec<u8>, v: i64) -> Result<()> {
    let v = encoded.write_i64::<LittleEndian>(v)?;
    Ok(v)
}

pub fn binary_write_i32(encoded: &mut Vec<u8>, v: i32) -> Result<()> {
    let v = encoded.write_i32::<LittleEndian>(v)?;
    Ok(v)
}

pub fn binary_write_f32(encoded: &mut Vec<u8>, v: f32) -> Result<()> {
    let v = encoded.write_f32::<LittleEndian>(v)?;
    Ok(v)
}

pub fn binary_write_i16(encoded: &mut Vec<u8>, v: i16) -> Result<()> {
    let v = encoded.write_i16::<LittleEndian>(v)?;
    Ok(v)
}

pub fn binary_write_i8(encoded: &mut Vec<u8>, v: i8) -> Result<()> {
    let v = encoded.write_i8(v)?;
    Ok(v)
}
pub fn binary_write_string(encoded: &mut Vec<u8>, v: &str) -> Result<()> {
    let bytes = v.as_bytes();
    binary_write_i16(encoded,bytes.len() as i16)?;
    encoded.extend_from_slice(bytes);
    Ok(())
}
