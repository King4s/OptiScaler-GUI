//! Small OS-information helpers (direct user32 externs, no extra crates).

/// True when the active keyboard layout's primary language is not US
/// English. OptiScaler's overlay hotkey is Insert, but on alternate
/// layouts (Danish, German, …) the overlay needs **Alt+Insert** — a
/// frequent "overlay doesn't work" support case, so the GUI shows a hint.
#[cfg(windows)]
pub fn non_us_keyboard() -> bool {
    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetKeyboardLayout(thread_id: u32) -> isize;
    }
    let hkl = unsafe { GetKeyboardLayout(0) };
    // Low word of the HKL is the language identifier; en-US is 0x0409.
    let lang_id = (hkl as usize) & 0xFFFF;
    lang_id != 0x0409
}

#[cfg(not(windows))]
pub fn non_us_keyboard() -> bool {
    false
}
