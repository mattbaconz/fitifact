//! Put a file on the clipboard the way Explorer's Copy does (CF_HDROP), so a
//! paste into a folder, a chat, or a mail draft attaches the file itself.

use std::path::Path;

#[cfg(windows)]
pub fn copy_file(path: &Path) -> std::io::Result<()> {
    use windows_sys::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, RegisterClipboardFormatW,
    };
    use windows_sys::Win32::System::Ole::{CF_HDROP, DROPEFFECT_COPY};

    struct Open;
    impl Drop for Open {
        fn drop(&mut self) {
            unsafe { CloseClipboard() };
        }
    }

    // Another program (a clipboard manager, say) may hold the clipboard briefly.
    let mut tries = 0;
    while unsafe { OpenClipboard(std::ptr::null_mut()) } == 0 {
        tries += 1;
        if tries == 10 {
            return Err(std::io::Error::last_os_error());
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let _open = Open;
    if unsafe { EmptyClipboard() } == 0 {
        return Err(std::io::Error::last_os_error());
    }
    set_data(u32::from(CF_HDROP), &drop_files(path))?;
    // Paste makes a copy; it never moves the user's file.
    let name: Vec<u16> = "Preferred DropEffect\0".encode_utf16().collect();
    let effect = unsafe { RegisterClipboardFormatW(name.as_ptr()) };
    if effect == 0 {
        return Err(std::io::Error::last_os_error());
    }
    set_data(effect, &DROPEFFECT_COPY.to_le_bytes())
}

#[cfg(not(windows))]
pub fn copy_file(_path: &Path) -> std::io::Result<()> {
    // ponytail: the desktop app ships on Windows only; add a pasteboard path when another OS ships.
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Copying a file is available on Windows only.",
    ))
}

/// A DROPFILES header, then the NUL-terminated UTF-16 path and a closing NUL.
#[cfg(windows)]
fn drop_files(path: &Path) -> Vec<u8> {
    use std::os::windows::ffi::OsStrExt;
    // Explorer does not accept the `\\?\` form that canonicalize returns.
    let path = dunce::simplified(path);
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&20u32.to_le_bytes()); // pFiles: the list starts after this header
    bytes.extend_from_slice(&[0; 12]); // pt and fNC: no drop point
    bytes.extend_from_slice(&1u32.to_le_bytes()); // fWide: UTF-16
    for unit in path.as_os_str().encode_wide().chain([0, 0]) {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    bytes
}

#[cfg(windows)]
fn set_data(format: u32, bytes: &[u8]) -> std::io::Result<()> {
    use windows_sys::Win32::Foundation::GlobalFree;
    use windows_sys::Win32::System::DataExchange::SetClipboardData;
    use windows_sys::Win32::System::Memory::{
        GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE,
    };

    unsafe {
        let memory = GlobalAlloc(GMEM_MOVEABLE, bytes.len());
        if memory.is_null() {
            return Err(std::io::Error::last_os_error());
        }
        let target = GlobalLock(memory);
        if target.is_null() {
            let error = std::io::Error::last_os_error();
            GlobalFree(memory);
            return Err(error);
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), target.cast::<u8>(), bytes.len());
        GlobalUnlock(memory);
        // On success the clipboard owns the memory.
        if SetClipboardData(format, memory).is_null() {
            let error = std::io::Error::last_os_error();
            GlobalFree(memory);
            return Err(error);
        }
    }
    Ok(())
}

#[cfg(all(test, windows))]
mod tests {
    use std::path::Path;

    #[test]
    fn drop_files_holds_one_plain_wide_path() {
        let bytes = super::drop_files(Path::new(r"\\?\C:\clips\clip.fitifact.mp4"));
        assert_eq!(bytes[..4], 20u32.to_le_bytes());
        assert_eq!(bytes[4..16], [0; 12]);
        assert_eq!(bytes[16..20], 1u32.to_le_bytes());
        let list: Vec<u8> = r"C:\clips\clip.fitifact.mp4"
            .encode_utf16()
            .chain([0, 0])
            .flat_map(u16::to_le_bytes)
            .collect();
        assert_eq!(bytes[20..], list);
    }
}
