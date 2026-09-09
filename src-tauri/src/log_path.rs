use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostOs {
    Mac,
    Windows,
    Other,
}

pub fn host_os() -> HostOs {
    if cfg!(target_os = "windows") {
        HostOs::Windows
    } else if cfg!(target_os = "macos") {
        HostOs::Mac
    } else {
        HostOs::Other
    }
}

pub fn player_log_path() -> Option<PathBuf> {
    player_log_path_for(host_os(), &home_dir()?)
}

pub fn player_prev_log_path(player_log: &Path) -> PathBuf {
    player_log.with_file_name("Player-prev.log")
}

pub fn player_log_path_for(os: HostOs, home: &Path) -> Option<PathBuf> {
    match os {
        HostOs::Mac => Some(home.join("Library/Logs/Wizards Of The Coast/MTGA/Player.log")),
        HostOs::Windows => Some(home.join("AppData/LocalLow/Wizards Of The Coast/MTGA/Player.log")),
        HostOs::Other => None,
    }
}

fn home_dir() -> Option<PathBuf> {
    dirs::home_dir()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn mac_path_matches_prd() {
        let path = player_log_path_for(HostOs::Mac, Path::new("/Users/chris")).unwrap();
        assert_eq!(
            path,
            Path::new("/Users/chris/Library/Logs/Wizards Of The Coast/MTGA/Player.log")
        );
    }

    #[test]
    fn windows_path_uses_locallow() {
        let path = player_log_path_for(HostOs::Windows, Path::new("/Users/chris")).unwrap();
        assert!(path.ends_with(Path::new(
            "AppData/LocalLow/Wizards Of The Coast/MTGA/Player.log"
        )));
    }

    #[test]
    fn prev_log_sits_beside_player_log() {
        let current = Path::new("/tmp/MTGA/Player.log");
        assert_eq!(
            player_prev_log_path(current),
            Path::new("/tmp/MTGA/Player-prev.log")
        );
    }
}
