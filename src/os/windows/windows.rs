use std::ffi::OsString;

use widestring::U16String;
use windows::Win32::Foundation::E_POINTER;
use windows::Win32::Foundation::S_FALSE;

use windows::Win32::Foundation::S_OK;

use windows::Win32::UI::Shell::*;

use windows::core::PCWSTR;
use windows::core::PWSTR;


pub enum FileType {
    Rust,
    Text
}

pub enum WindowsError {
    BufferTooSmall,
    False,
    Unknown,
    UndefinedType
}


fn get_editor(extension: &str) -> Result<OsString, WindowsError> {

    let mut r_ptr = [0u16; 1024];
    let mut size = 1024;

    let extension = U16String::from_str(extension);

    //safety: we cannot overrun the buffer since we are using the NOTRUNCATE OPTION with a known buffer size
    let result = unsafe {
        let hresult = AssocQueryStringW(
            ASSOCF_NOTRUNCATE | ASSOCF_REMAPRUNDLL | ASSOCF_INIT_FOR_FILE | ASSOCF_INIT_FIXED_PROGID, 
            ASSOCSTR_EXECUTABLE,
            PCWSTR::from_raw(extension.as_ptr()),
            PCWSTR::null(),
            PWSTR::from_raw(r_ptr.as_mut_ptr()),
            &mut size);

        match hresult {
            S_OK => Ok(U16String::from_vec(r_ptr).to_os_string()),
            
            S_FALSE => Err(WindowsError::False),
            E_POINTER => Err(WindowsError::BufferTooSmall),
            windows::core::HRESULT(-2147023741) => Err(WindowsError::UndefinedType),
            _ => Err(WindowsError::Unknown)
        }

    };

    result

}

pub fn detect_type_editor(t: FileType) -> Result<OsString, WindowsError> {

    match t {
        FileType::Rust => get_editor(".rs"),
        FileType::Text => get_editor(".txt"),
    }

}