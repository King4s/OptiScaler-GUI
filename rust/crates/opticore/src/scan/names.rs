//! Name heuristics ported from the Python scanner (game_scanner.py):
//! CamelCase splitting, WindowsApps (Appx) package-name parsing, and the
//! launcher/non-game keyword filters.

/// Split CamelCase/PascalCase and letter↔digit boundaries into words.
/// Port of `GameScanner._split_camel_case`.
pub fn split_camel_case(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut out = String::with_capacity(name.len() + 8);
    for (i, &c) in chars.iter().enumerate() {
        if i > 0 {
            let prev = chars[i - 1];
            let boundary = (prev.is_lowercase() && c.is_uppercase())
                || (prev.is_alphabetic() && c.is_ascii_digit())
                || (prev.is_ascii_digit() && c.is_alphabetic());
            if boundary {
                out.push(' ');
            }
        }
        out.push(c);
    }
    out
}

/// Title-case each word (Python's `str.title()` for our ASCII-ish names).
pub fn title_case(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut at_word_start = true;
    for c in s.chars() {
        if c.is_alphanumeric() {
            if at_word_start {
                out.extend(c.to_uppercase());
            } else {
                out.extend(c.to_lowercase());
            }
            at_word_start = false;
        } else {
            out.push(c);
            at_word_start = true;
        }
    }
    out
}

/// Replace underscores/hyphens with spaces and title-case — the folder-name
/// fallback used across the Python scanners.
pub fn folder_name_to_title(folder_name: &str) -> String {
    title_case(&folder_name.replace(['_', '-'], " "))
}

/// Extract a readable game name from a WindowsApps folder
/// (`Publisher.AppName_Version_Arch_Hash`). Port of `_parse_appx_package_name`.
pub fn parse_appx_package_name(folder_name: &str) -> String {
    // Strip everything from the first "_<digits>.<digit>..." (version suffix)
    let mut base = folder_name;
    let bytes: Vec<char> = folder_name.chars().collect();
    for (i, &c) in bytes.iter().enumerate() {
        if c == '_' {
            // does a version number follow? (digits then '.')
            let rest: String = bytes[i + 1..].iter().collect();
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if !digits.is_empty() && rest[digits.len()..].starts_with('.') {
                base = &folder_name[..folder_name
                    .char_indices()
                    .nth(i)
                    .map(|(b, _)| b)
                    .unwrap_or(folder_name.len())];
                break;
            }
        }
    }
    // Take the AppName after the last dot (drop the publisher)
    let base = base.rsplit('.').next().unwrap_or(base);
    split_camel_case(base).trim().to_string()
}

/// Known non-game keyword fragments in Appx package names.
/// Port of `_APPX_NON_GAME_KEYWORDS`.
const APPX_NON_GAME_KEYWORDS: &[&str] = &[
    "vclibs",
    "runtime",
    "framework",
    "xaml",
    "webpimage",
    "hevcvideo",
    "heifimage",
    "mpeg2video",
    "vp9video",
    "webmedia",
    "codec",
    "videoextension",
    "imageextension",
    "storepurchase",
    "storeengagement",
    "store.engagement",
    "directxruntime",
    "net.native",
    "appruntime",
    "winappruntime",
    "windowsappruntime",
    "gamingservices",
    "xboxgamingoverlay",
    "gamingapp",
    "xboxidentityprovider",
    "xboxdevices",
    "xbox.tcui",
    "bingwallpaper",
    "getstarted",
    "gethelp",
    "startexperiencesapp",
    "commandpalette",
    "webexperience",
    "crossdevice",
    "sechealthui",
    "screensketch",
    "widgetsplatform",
    "foundrylocal",
    "applicationcompatibility",
    "avcencoder",
    "windowsstore",
    "storepurchaseapp",
    "yourphone",
    "windowsterminal",
    "windowsnotepad",
    "windowscalculator",
    "windowscamera",
    "windowsphotos",
    "zunemusic",
    "microsoftmahjong",
    "candycrush",
    "linkedinfor",
    "whatsappdesktop",
    "clipchamp",
    "dolbyaccess",
    "dtssound",
    "armourycrate",
    "lgmonitor",
    "realtekaudio",
    "tobiieyetracking",
    "speedtest",
    "hdrcalibration",
    "camostudio",
    "icloud",
    "itunes",
    "applemusic",
    "appletv",
    "primevideo",
    "vidstok",
    "gameassist",
    "paint",
    "photos",
    "minecraftuwp",
    "minecraftlauncher",
    "facebook",
    "netflix",
    "desktopappinstaller",
    "languageexperience",
    "solitairecollection",
    "onedrivesync",
    "powertoys",
    "microsoftedge",
    "preordercontent",
    "cloudspremium",
    "anthropic",
    "claude_",
];

/// True if the WindowsApps folder could be a game. Port of `_is_appx_game_candidate`.
pub fn is_appx_game_candidate(folder_name: &str) -> bool {
    let lower = folder_name.to_lowercase();
    if !folder_name.contains('_') {
        return false;
    }
    // Skip packages whose AppName part looks like a hex identifier
    let name_part = lower.split('_').next().unwrap_or("");
    let app_part = name_part.rsplit('.').next().unwrap_or("");
    let squeezed: String = app_part.chars().filter(|c| *c != ' ').collect();
    if squeezed.len() >= 6 && squeezed.chars().all(|c| c.is_ascii_hexdigit()) {
        return false;
    }
    !APPX_NON_GAME_KEYWORDS.iter().any(|kw| lower.contains(kw))
}

/// Launcher/redistributable entries filtered out of scan results.
/// Port of the `_launcher_keywords` filter in `scan_games`.
const LAUNCHER_KEYWORDS: &[&str] = &[
    "launcher",
    "redistributable",
    "directx",
    "vcredist",
    "dotnet",
    "steamworks common",
];

pub fn is_launcher_entry(game_name: &str) -> bool {
    let lower = game_name.to_lowercase();
    LAUNCHER_KEYWORDS.iter().any(|kw| lower.contains(kw))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_camel_case() {
        assert_eq!(split_camel_case("HaloInfinite"), "Halo Infinite");
        assert_eq!(split_camel_case("Game2Exe"), "Game 2 Exe");
        assert_eq!(split_camel_case("already split"), "already split");
    }

    #[test]
    fn parses_appx_names() {
        assert_eq!(
            parse_appx_package_name("Hoodedhorse.ManorLords_1.5.1.0_x64__abc123"),
            "Manor Lords"
        );
        assert_eq!(
            parse_appx_package_name("Microsoft.HaloInfinite_6.10021.18539.0_x64__8wekyb3d8bbwe"),
            "Halo Infinite"
        );
    }

    #[test]
    fn appx_candidate_filter() {
        assert!(is_appx_game_candidate(
            "Hoodedhorse.ManorLords_1.5.1.0_x64__abc"
        ));
        assert!(!is_appx_game_candidate("Deleted")); // no underscore
        assert!(!is_appx_game_candidate(
            "Microsoft.VCLibs.140.00_14.0.33519.0_x64__8wekyb3d8bbwe"
        ));
        assert!(!is_appx_game_candidate(
            "1Ed5Aea5.4160926B82Db_1.0.0.0_x64__xyz"
        )); // hex id
    }

    #[test]
    fn launcher_filter() {
        assert!(is_launcher_entry("Epic Games Launcher"));
        assert!(is_launcher_entry("DirectX Redist"));
        assert!(!is_launcher_entry("Death Stranding"));
    }

    #[test]
    fn folder_name_titles() {
        assert_eq!(
            folder_name_to_title("dead_space-remake"),
            "Dead Space Remake"
        );
    }
}
