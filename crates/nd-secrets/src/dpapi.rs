#![allow(unsafe_code)]

use windows_sys::Win32::Foundation::LocalFree;
use windows_sys::Win32::Security::Cryptography::{
    CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
};

use crate::{Result, other};

const ENTROPY: &[u8] = b"app.nicedb.desktop";

fn blob(data: &[u8]) -> Result<CRYPT_INTEGER_BLOB> {
    Ok(CRYPT_INTEGER_BLOB {
        cbData: u32::try_from(data.len()).map_err(other)?,
        pbData: data.as_ptr().cast_mut(),
    })
}

fn run(
    data: &[u8],
    call: unsafe fn(&CRYPT_INTEGER_BLOB, &CRYPT_INTEGER_BLOB, &mut CRYPT_INTEGER_BLOB) -> i32,
) -> Result<Vec<u8>> {
    let input = blob(data)?;
    let entropy = blob(ENTROPY)?;
    let mut out = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    if unsafe { call(&input, &entropy, &mut out) } == 0 {
        return Err(other(std::io::Error::last_os_error()));
    }
    let bytes = unsafe { std::slice::from_raw_parts(out.pbData, out.cbData as usize) }.to_vec();
    unsafe { LocalFree(out.pbData.cast()) };
    Ok(bytes)
}

unsafe fn protect_raw(
    i: &CRYPT_INTEGER_BLOB,
    e: &CRYPT_INTEGER_BLOB,
    o: &mut CRYPT_INTEGER_BLOB,
) -> i32 {
    unsafe {
        CryptProtectData(
            i,
            std::ptr::null(),
            e,
            std::ptr::null(),
            std::ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            o,
        )
    }
}

unsafe fn unprotect_raw(
    i: &CRYPT_INTEGER_BLOB,
    e: &CRYPT_INTEGER_BLOB,
    o: &mut CRYPT_INTEGER_BLOB,
) -> i32 {
    unsafe {
        CryptUnprotectData(
            i,
            std::ptr::null_mut(),
            e,
            std::ptr::null(),
            std::ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            o,
        )
    }
}

pub(crate) fn protect(data: &[u8]) -> Result<Vec<u8>> {
    run(data, protect_raw)
}

pub(crate) fn unprotect(data: &[u8]) -> Result<Vec<u8>> {
    run(data, unprotect_raw)
}
