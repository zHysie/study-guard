#[cfg(windows)]
pub fn system_idle_seconds() -> Result<u64, String> {
    use std::mem::size_of;
    use windows::Win32::System::SystemInformation::GetTickCount;
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};

    let mut info = LASTINPUTINFO {
        cbSize: size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };

    let ok = unsafe { GetLastInputInfo(&mut info) }.as_bool();
    if !ok {
        return Err("GetLastInputInfo failed".into());
    }

    let now = unsafe { GetTickCount() };
    Ok(now.saturating_sub(info.dwTime) as u64 / 1000)
}

#[cfg(not(windows))]
pub fn system_idle_seconds() -> Result<u64, String> {
    Ok(0)
}
