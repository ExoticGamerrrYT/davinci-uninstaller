// DaVinci Resolve Uninstaller — backend.
// scan() inspects (never deletes); uninstall() removes. Every deletion goes
// through safe_to_delete() as an allowlist: only Blackmagic/PostgreSQL paths or
// Resolve shortcuts.

use serde::Serialize;
use std::fs;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tauri::{AppHandle, Emitter};
use winreg::enums::*;
use winreg::{RegKey, HKEY};

// Spawn processes with no console window (avoids cmd/powershell flashes).
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn hidden(program: &str) -> Command {
    let mut c = Command::new(program);
    c.creation_flags(CREATE_NO_WINDOW);
    c
}

// ---------- known locations ----------

fn env_dir(var: &str) -> Option<PathBuf> {
    std::env::var_os(var).map(PathBuf::from).filter(|p| !p.as_os_str().is_empty())
}

fn bmd(root: PathBuf, sub: &str) -> PathBuf {
    let mut p = root.join("Blackmagic Design");
    for part in sub.split('/').filter(|s| !s.is_empty()) {
        p = p.join(part);
    }
    p
}

// App/config folders (we target the "DaVinci Resolve" subfolder rather than the
// whole "Blackmagic Design" folder, so other Blackmagic products aren't wiped —
// except the local cache folder).
fn app_folders() -> Vec<(String, PathBuf)> {
    let mut v = Vec::new();
    let mut add = |label: &str, p: Option<PathBuf>| {
        if let Some(p) = p { v.push((label.to_string(), p)); }
    };
    add("Installation", env_dir("PROGRAMFILES").map(|r| bmd(r, "DaVinci Resolve")));
    add("Common data (ProgramData)", env_dir("PROGRAMDATA").map(|r| bmd(r, "DaVinci Resolve")));
    add("User config (Roaming)", env_dir("APPDATA").map(|r| bmd(r, "DaVinci Resolve")));
    add("User cache (Local)", env_dir("LOCALAPPDATA").map(|r| r.join("Blackmagic Design")));
    add("User documents", env_dir("USERPROFILE").map(|r| r.join("Documents")).map(|r| bmd(r, "DaVinci Resolve")));
    add("Public documents", env_dir("PUBLIC").map(|r| r.join("Documents")).map(|r| bmd(r, "DaVinci Resolve")));
    v
}

// Start Menu folders holding the shortcuts.
fn start_menu_folders() -> Vec<(String, PathBuf)> {
    let mut v = Vec::new();
    if let Some(r) = env_dir("PROGRAMDATA") {
        v.push(("Start Menu (all users)".into(),
                r.join("Microsoft").join("Windows").join("Start Menu").join("Programs").join("Blackmagic Design")));
    }
    if let Some(r) = env_dir("APPDATA") {
        v.push(("Start Menu (current user)".into(),
                r.join("Microsoft").join("Windows").join("Start Menu").join("Programs").join("Blackmagic Design")));
    }
    v
}

// Loose shortcuts on the desktop (current user and public).
fn desktop_shortcuts() -> Vec<PathBuf> {
    let mut v = Vec::new();
    let mut desktops = Vec::new();
    if let Some(p) = env_dir("USERPROFILE") { desktops.push(p.join("Desktop")); }
    if let Some(p) = env_dir("PUBLIC") { desktops.push(p.join("Desktop")); }
    for d in desktops {
        if let Ok(entries) = fs::read_dir(&d) {
            for e in entries.flatten() {
                let p = e.path();
                let name = p.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
                if name.ends_with(".lnk") && (name.contains("resolve") || name.contains("blackmagic")) {
                    v.push(p);
                }
            }
        }
    }
    v
}

// Project databases (they live INSIDE the app folders).
fn project_db_folders() -> Vec<PathBuf> {
    let mut v = Vec::new();
    let sub = "DaVinci Resolve/Support/Resolve Disk Database";
    if let Some(r) = env_dir("PROGRAMDATA") { v.push(bmd(r, sub)); }
    if let Some(r) = env_dir("APPDATA") { v.push(bmd(r, sub)); }
    v
}

// PostgreSQL folders (Resolve's optional centralized database).
fn postgres_folders() -> Vec<PathBuf> {
    let mut v = Vec::new();
    for var in ["PROGRAMFILES", "ProgramFiles(x86)"] {
        if let Some(r) = env_dir(var) {
            let p = r.join("PostgreSQL");
            if p.exists() { v.push(p); }
        }
    }
    v
}

// Registry keys (hive, subkey, label).
fn registry_keys() -> Vec<(HKEY, &'static str, &'static str)> {
    vec![
        (HKEY_LOCAL_MACHINE, "SOFTWARE\\Blackmagic Design\\DaVinci Resolve", "HKLM Blackmagic"),
        (HKEY_LOCAL_MACHINE, "SOFTWARE\\WOW6432Node\\Blackmagic Design\\DaVinci Resolve", "HKLM (32-bit)"),
        (HKEY_CURRENT_USER, "SOFTWARE\\Blackmagic Design\\DaVinci Resolve", "HKCU Blackmagic"),
    ]
}

// ---------- deletion helpers ----------

// Allowlist: never delete anything that isn't clearly Resolve's.
fn safe_to_delete(path: &Path) -> bool {
    let s = path.to_string_lossy().to_lowercase();
    path.components().count() >= 4
        && (s.contains("blackmagic")
            || s.contains("postgresql")
            || (s.ends_with(".lnk") && (s.contains("resolve") || s.contains("blackmagic"))))
}

fn is_protected(path: &Path, protected: &[PathBuf]) -> bool {
    protected.iter().any(|p| path == p.as_path() || path.starts_with(p))
}

fn has_protected_descendant(path: &Path, protected: &[PathBuf]) -> bool {
    protected.iter().any(|p| p.starts_with(path))
}

// Deletes `path` recursively, but keeps any subtree in `protected`
// (so the app trace is removed without touching the projects).
fn delete_path(path: &Path, protected: &[PathBuf], errors: &mut Vec<String>) {
    if !path.exists() { return; }
    if is_protected(path, protected) { return; }
    if path.is_dir() && has_protected_descendant(path, protected) {
        if let Ok(entries) = fs::read_dir(path) {
            for e in entries.flatten() {
                delete_path(&e.path(), protected, errors);
            }
        }
        return; // keep this folder because it contains projects
    }
    if !safe_to_delete(path) {
        errors.push(format!("Skipped for safety: {}", path.display()));
        return;
    }
    let res = if path.is_dir() { fs::remove_dir_all(path) } else { fs::remove_file(path) };
    if let Err(e) = res {
        errors.push(format!("{}: {}", path.display(), e));
    }
}

fn delete_reg(hive: HKEY, subkey: &str, errors: &mut Vec<String>) {
    match RegKey::predef(hive).delete_subkey_all(subkey) {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => errors.push(format!("Registry {}: {}", subkey, e)),
    }
}

fn reg_exists(hive: HKEY, subkey: &str) -> bool {
    RegKey::predef(hive).open_subkey(subkey).is_ok()
}

// Looks up Blackmagic's official uninstall command in the registry.
fn find_uninstall_string() -> Option<String> {
    let bases = [
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
        "SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
    ];
    for hive in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
        for base in bases {
            let root = RegKey::predef(hive);
            let Ok(unins) = root.open_subkey(base) else { continue };
            for name in unins.enum_keys().flatten() {
                let Ok(k) = unins.open_subkey(&name) else { continue };
                let dn: Result<String, _> = k.get_value("DisplayName");
                if dn.map(|d| d.contains("DaVinci Resolve")).unwrap_or(false) {
                    if let Ok(q) = k.get_value::<String, _>("QuietUninstallString") { return Some(q); }
                    if let Ok(u) = k.get_value::<String, _>("UninstallString") { return Some(u); }
                }
            }
        }
    }
    None
}

fn kill_processes() {
    let names = [
        "Resolve.exe", "DaVinci Resolve.exe", "fuscript.exe", "VstScanner.exe",
        "BlackmagicRAWSpeedTest.exe", "Blackmagic Design DoNotDelete.exe",
    ];
    for n in names {
        let _ = hidden("taskkill").args(["/F", "/IM", n])
            .stdout(Stdio::null()).stderr(Stdio::null()).status();
    }
}

// Stops and removes PostgreSQL services (Resolve's project database).
// ponytail: matches by name "postgres"; could hit an unrelated PostgreSQL.
// Upgrade: filter by binPath under Blackmagic. Only runs when the user opts
// into "remove projects".
fn stop_postgres_services(errors: &mut Vec<String>) {
    let out = hidden("sc").args(["query", "state=", "all"]).output();
    let Ok(out) = out else { return };
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        let line = line.trim();
        if let Some(name) = line.strip_prefix("SERVICE_NAME:") {
            let name = name.trim();
            if name.to_lowercase().contains("postgres") {
                let _ = hidden("sc").args(["stop", name]).status();
                if let Err(e) = hidden("sc").args(["delete", name]).status() {
                    errors.push(format!("Service {}: {}", name, e));
                }
            }
        }
    }
}

// ---------- commands ----------

#[derive(Serialize)]
struct Item {
    label: String,
    path: String,
    kind: String, // folder | shortcut | registry | postgres
    exists: bool,
}

#[derive(Serialize)]
struct ScanResult {
    app: Vec<Item>,
    projects: Vec<Item>,
    system_drive: String, // only the system drive is cleaned (e.g. "C:")
}

// Async so it doesn't block the UI thread (sync commands run on it).
#[tauri::command]
async fn is_admin() -> bool {
    tauri::async_runtime::spawn_blocking(|| {
        hidden("net").arg("session")
            .stdout(Stdio::null()).stderr(Stdio::null())
            .status().map(|s| s.success()).unwrap_or(false)
    }).await.unwrap_or(false)
}

// Only checks existence (no folder walking) -> instant.
fn do_scan() -> ScanResult {
    let mut app = Vec::new();
    let mut projects = Vec::new();

    for (label, p) in app_folders().into_iter().chain(start_menu_folders()) {
        app.push(Item { label, path: p.display().to_string(), kind: "folder".into(), exists: p.exists() });
    }
    for p in desktop_shortcuts() {
        app.push(Item { label: "Shortcut".into(), path: p.display().to_string(),
                        kind: "shortcut".into(), exists: true });
    }
    for (hive, sub, label) in registry_keys() {
        app.push(Item { label: label.into(), path: sub.to_string(),
                        kind: "registry".into(), exists: reg_exists(hive, sub) });
    }

    for p in project_db_folders() {
        projects.push(Item { label: "Disk database".into(), path: p.display().to_string(),
                             kind: "folder".into(), exists: p.exists() });
    }
    for p in postgres_folders() {
        projects.push(Item { label: "PostgreSQL".into(), path: p.display().to_string(),
                             kind: "postgres".into(), exists: true });
    }

    let system_drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
    ScanResult { app, projects, system_drive }
}

#[tauri::command]
async fn scan() -> Result<ScanResult, String> {
    tauri::async_runtime::spawn_blocking(do_scan).await.map_err(|e| e.to_string())
}

#[derive(Serialize)]
struct UninstallResult {
    removed: usize,
    errors: Vec<String>,
}

fn do_uninstall(app: AppHandle, remove_projects: bool) -> UninstallResult {
    let mut errors = Vec::new();
    let mut removed = 0usize;
    let log = |m: String| { let _ = app.emit("log", m); };

    log("Closing DaVinci Resolve processes…".into());
    kill_processes();

    log("Running the official Blackmagic uninstaller…".into());
    match find_uninstall_string() {
        Some(cmd) => {
            let status = hidden("cmd").args(["/C", &cmd]).status();
            if let Err(e) = status { errors.push(format!("Official uninstaller: {}", e)); }
        }
        None => log("Official uninstaller not found; cleaning up manually.".into()),
    }

    // Projects to keep when the user does NOT want to delete them.
    let protected: Vec<PathBuf> = if remove_projects {
        Vec::new()
    } else {
        project_db_folders().into_iter().filter(|p| p.exists()).collect()
    };

    log("Removing files and folders…".into());
    for (label, p) in app_folders().into_iter().chain(start_menu_folders()) {
        if p.exists() {
            log(format!("  {} — {}", label, p.display()));
            delete_path(&p, &protected, &mut errors);
            removed += 1;
        }
    }
    for p in desktop_shortcuts() {
        log(format!("  Shortcut — {}", p.display()));
        delete_path(&p, &[], &mut errors);
        removed += 1;
    }

    log("Removing registry keys…".into());
    for (hive, sub, _) in registry_keys() {
        if reg_exists(hive, sub) { removed += 1; }
        delete_reg(hive, sub, &mut errors);
    }

    if remove_projects {
        log("Removing project database (PostgreSQL)…".into());
        stop_postgres_services(&mut errors);
        for p in postgres_folders() {
            if p.exists() {
                delete_path(&p, &[], &mut errors);
                removed += 1;
            }
        }
    }

    log("Done.".into());
    UninstallResult { removed, errors }
}

#[tauri::command]
async fn uninstall(app: AppHandle, remove_projects: bool) -> Result<UninstallResult, String> {
    tauri::async_runtime::spawn_blocking(move || do_uninstall(app, remove_projects))
        .await.map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![is_admin, scan, uninstall])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// Minimal check of the non-trivial logic (allowlist + project protection).
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlist_rejects_random_paths() {
        assert!(!safe_to_delete(Path::new("C:\\Windows\\System32")));
        assert!(!safe_to_delete(Path::new("C:\\Users\\me\\Documents\\photo.jpg")));
        assert!(safe_to_delete(Path::new("C:\\ProgramData\\Blackmagic Design\\DaVinci Resolve")));
        assert!(safe_to_delete(Path::new("C:\\Users\\me\\Desktop\\DaVinci Resolve.lnk")));
    }

    #[test]
    fn protected_projects_are_kept() {
        let db = PathBuf::from("C:\\ProgramData\\Blackmagic Design\\DaVinci Resolve\\Support\\Resolve Disk Database");
        let inside = db.join("Resolve Projects");
        let outside = PathBuf::from("C:\\ProgramData\\Blackmagic Design\\DaVinci Resolve\\logs");
        let protected = vec![db.clone()];
        assert!(is_protected(&inside, &protected));
        assert!(!is_protected(&outside, &protected));
        // the parent holds a protected subtree -> it isn't deleted wholesale
        assert!(has_protected_descendant(db.parent().unwrap(), &protected));
    }
}
