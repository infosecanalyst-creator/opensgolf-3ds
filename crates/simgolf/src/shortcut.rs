//! The launcher's *Desktop shortcut* (the port's own; docs/PLAYING.md "The launcher"): a shortcut on the desktop that
//! starts this copy of the game with `--play`, which goes straight to the title menu with the settings the launcher saved
//! (the launcher's tick stays as it is for other starts). Windows: a .lnk made by the system's own shell (PowerShell and
//! WScript.Shell); Linux: a .desktop file on the desktop (the XDG desktop folder) made runnable; macOS: an alias of
//! OpenSGolf.app made by the Finder (an alias cannot carry `--play`, so it starts as the app does).
#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
use std::path::{Path, PathBuf};

/// The command-line switch a shortcut starts the game with: no launcher this time.
pub const PLAY_ARG: &str = "--play";

/// A Linux desktop entry for the program at `exe` (with `--play`), its icon `icon` if there is one.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn desktop_entry(exe: &str, icon: Option<&str>) -> String {
    let q = |s: &str| format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""));
    let mut t = format!(
        "[Desktop Entry]\nType=Application\nName=OpenSGolf\nComment=The classic golf course builder, rebuilt\nExec={} {PLAY_ARG}\nTerminal=false\nCategories=Game;SimulationGame;\n",
        q(exe)
    );
    if let Some(i) = icon {
        t += &format!("Icon={i}\n");
    }
    t
}

/// Makes the shortcut; the place it was made, or why not.
#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
pub fn make() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    // an AppImage runs from a temporary mount: the shortcut starts the AppImage itself
    let exe = std::env::var_os("APPIMAGE").map(PathBuf::from).unwrap_or(exe);
    make_for(&exe)
}

#[cfg(target_os = "windows")]
fn make_for(exe: &Path) -> Result<PathBuf, String> {
    let ps = |s: &str| s.replace('\'', "''");
    let dir = exe.parent().unwrap_or(Path::new("."));
    let script = format!(
        "$d=[Environment]::GetFolderPath('Desktop');$p=Join-Path $d 'OpenSGolf.lnk';\
         $s=(New-Object -ComObject WScript.Shell).CreateShortcut($p);$s.TargetPath='{e}';$s.Arguments='{PLAY_ARG}';\
         $s.WorkingDirectory='{w}';$s.IconLocation='{e},0';$s.Save();Write-Output $p",
        e = ps(&exe.to_string_lossy()),
        w = ps(&dir.to_string_lossy())
    );
    use std::os::windows::process::CommandExt;
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(PathBuf::from(String::from_utf8_lossy(&out.stdout).trim()))
}

#[cfg(target_os = "macos")]
fn make_for(exe: &Path) -> Result<PathBuf, String> {
    // OpenSGolf.app/Contents/MacOS/opensgolf: the app itself
    let app = exe.ancestors().find(|p| p.extension().is_some_and(|e| e == "app")).ok_or("not started from OpenSGolf.app")?;
    let a = app.to_string_lossy().replace('\\', "\\\\").replace('"', "\\\"");
    let script = format!("tell application \"Finder\" to make alias file to (POSIX file \"{a}\") at desktop");
    let out = std::process::Command::new("osascript").args(["-e", &script]).output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    Ok(home.join("Desktop").join(app.file_name().unwrap_or_default()))
}

#[cfg(all(not(any(target_os = "windows", target_os = "macos")), not(any(target_arch = "wasm32", target_os = "android"))))]
fn make_for(exe: &Path) -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME").map(PathBuf::from).ok_or("no home folder")?;
    // the desktop folder as the desktop names it (xdg-user-dir), else ~/Desktop
    let desk = std::process::Command::new("xdg-user-dir")
        .arg("DESKTOP")
        .output()
        .ok()
        .map(|o| PathBuf::from(String::from_utf8_lossy(&o.stdout).trim()))
        .filter(|p| p.is_dir() && *p != home)
        .unwrap_or_else(|| home.join("Desktop"));
    std::fs::create_dir_all(&desk).map_err(|e| e.to_string())?;
    let icon = exe.parent().map(|d| d.join("opensgolf.png")).filter(|p| p.is_file());
    let text = desktop_entry(&exe.to_string_lossy(), icon.as_deref().and_then(|p| p.to_str()));
    let p = desk.join("opensgolf.desktop");
    std::fs::write(&p, text).map_err(|e| e.to_string())?;
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755));
    // GNOME asks for a desktop file to be trusted before it starts; mark it so where gio is there
    let _ = std::process::Command::new("gio").args(["set", &p.to_string_lossy(), "metadata::trusted", "true"]).output();
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_desktop_entry_starts_the_game_without_the_launcher() {
        let t = desktop_entry("/home/me/Games/OpenSGolf/opensgolf", Some("/home/me/Games/OpenSGolf/opensgolf.png"));
        assert!(t.contains("Exec=\"/home/me/Games/OpenSGolf/opensgolf\" --play\n"));
        assert!(t.contains("Icon=/home/me/Games/OpenSGolf/opensgolf.png\n"));
        assert!(desktop_entry("/a \"b\"/x", None).contains("Exec=\"/a \\\"b\\\"/x\" --play"));
    }
}
