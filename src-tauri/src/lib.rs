use serde::Serialize;
use std::{
    fs,
    net::{TcpStream, ToSocketAddrs},
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::Duration,
};
use tauri::{AppHandle, Manager};

const OPENLIST_BINS: [&str; 2] = ["openlist.exe", "openlist-x86_64-pc-windows-msvc.exe"];
const OPENLIST_URL: &str = "http://127.0.0.1:5244";
const CREDENTIAL_TARGET_PREFIX: &str = "OpenList Explorer:OpenList API Token:";
const LEGACY_CREDENTIAL_TARGET: &str = "OpenList Explorer:OpenList API Token";
const CREDENTIAL_USER: &str = "OpenList Explorer";
const BUILTIN_ADMIN_PASSWORD_ID: &str = "builtin-local-admin-password";

#[derive(Serialize)]
struct BuiltinOpenListStatus {
    available: bool,
    running: bool,
    server_url: String,
    binary_path: Option<String>,
    data_dir: Option<String>,
    message: String,
}

#[derive(Serialize)]
struct BuiltinOpenListSession {
    server_url: String,
    token: String,
    data_dir: String,
    admin_username: String,
    admin_password: String,
}

#[derive(Serialize)]
struct LocalDownloadResult {
    path: String,
}

fn is_port_open() -> bool {
    ("127.0.0.1", 5244)
        .to_socket_addrs()
        .ok()
        .and_then(|mut addrs| addrs.next())
        .and_then(|addr| TcpStream::connect_timeout(&addr, Duration::from_millis(600)).ok())
        .is_some()
}

fn app_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("无法获取应用数据目录: {error}"))?
        .join("openlist");
    fs::create_dir_all(&dir).map_err(|error| format!("无法创建 OpenList 数据目录: {error}"))?;
    Ok(dir)
}

fn candidate_binaries(app: &AppHandle) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(resource_dir) = app.path().resource_dir() {
        for binary_name in OPENLIST_BINS {
            paths.push(resource_dir.join(binary_name));
            paths.push(resource_dir.join("binaries").join(binary_name));
        }
    }

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            for binary_name in OPENLIST_BINS {
                paths.push(parent.join(binary_name));
                paths.push(parent.join("binaries").join(binary_name));
            }
        }
    }

    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        for binary_name in OPENLIST_BINS {
            paths.push(PathBuf::from(&manifest_dir).join("binaries").join(binary_name));
        }
    }

    paths
}

fn openlist_binary(app: &AppHandle) -> Option<PathBuf> {
    candidate_binaries(app).into_iter().find(|path| path.exists())
}

fn hidden_command(path: &PathBuf) -> Command {
    let mut command = Command::new(path);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
}

fn read_admin_token(binary: &PathBuf, data_dir: &PathBuf) -> Result<String, String> {
    let output = hidden_command(binary)
        .args(["admin", "token", "--data"])
        .arg(data_dir)
        .output()
        .map_err(|error| format!("无法读取 OpenList Token: {error}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}\n{stderr}");

    combined
        .lines()
        .find_map(|line| line.trim().strip_prefix("Admin token:").map(str::trim))
        .filter(|token| !token.is_empty())
        .map(str::to_string)
        .ok_or_else(|| "未能从 OpenList 输出中读取 Token".to_string())
}

fn parse_admin_password(output: &str) -> Option<String> {
    output.lines().rev().find_map(|line| {
        let line = line.trim();
        line.strip_prefix("password:")
            .or_else(|| line.split("initial password is:").nth(1))
            .map(str::trim)
            .filter(|password| !password.is_empty())
            .map(str::to_string)
    })
}

fn ensure_builtin_admin_password(binary: &PathBuf, data_dir: &PathBuf) -> Result<String, String> {
    if let Some(password) = read_openlist_token(BUILTIN_ADMIN_PASSWORD_ID.to_string())? {
        return Ok(password);
    }

    let output = hidden_command(binary)
        .args(["admin", "random", "--data"])
        .arg(data_dir)
        .output()
        .map_err(|error| format!("无法生成 OpenList 管理员密码: {error}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}\n{stderr}");
    let password = parse_admin_password(&combined)
        .ok_or_else(|| "未能从 OpenList 输出中读取管理员密码".to_string())?;
    save_openlist_token(BUILTIN_ADMIN_PASSWORD_ID.to_string(), password.clone())?;
    Ok(password)
}

fn default_download_dir() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .map(|path| path.join("Downloads"))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

fn clean_filename(name: &str) -> String {
    let cleaned = name
        .chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            _ => ch,
        })
        .collect::<String>()
        .trim()
        .trim_matches('.')
        .to_string();

    if cleaned.is_empty() {
        "download.bin".to_string()
    } else {
        cleaned
    }
}

fn unique_path(dir: &Path, filename: &str) -> PathBuf {
    let candidate = dir.join(filename);
    if !candidate.exists() {
        return candidate;
    }

    let path = Path::new(filename);
    let stem = path.file_stem().and_then(|value| value.to_str()).unwrap_or("download");
    let ext = path.extension().and_then(|value| value.to_str());

    for index in 1..1000 {
        let next_name = match ext {
            Some(ext) => format!("{stem} ({index}).{ext}"),
            None => format!("{stem} ({index})"),
        };
        let next = dir.join(next_name);
        if !next.exists() {
            return next;
        }
    }

    dir.join(filename)
}

fn clean_relative_path(relative_path: &str, fallback_filename: &str) -> PathBuf {
    let mut path = PathBuf::new();
    for part in relative_path.split(['/', '\\']).map(str::trim).filter(|part| !part.is_empty()) {
        if part == "." || part == ".." {
            continue;
        }
        path.push(clean_filename(part));
    }

    if path.as_os_str().is_empty() {
        PathBuf::from(clean_filename(fallback_filename))
    } else {
        path
    }
}

#[tauri::command]
fn default_download_path() -> String {
    default_download_dir().display().to_string()
}

#[tauri::command]
fn download_to_local(url: String, filename: String, download_dir: Option<String>) -> Result<LocalDownloadResult, String> {
    let dir = download_dir
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(default_download_dir);
    fs::create_dir_all(&dir).map_err(|error| format!("无法创建下载目录: {error}"))?;
    let path = unique_path(&dir, &clean_filename(&filename));

    let status = Command::new("curl.exe")
        .args(["-L", "-f", "--retry", "2", "-o"])
        .arg(&path)
        .arg(&url)
        .status()
        .map_err(|error| format!("无法启动下载工具 curl.exe: {error}"))?;

    if !status.success() {
        let _ = fs::remove_file(&path);
        return Err("下载失败，请检查直链是否可访问".to_string());
    }

    Ok(LocalDownloadResult {
        path: path.display().to_string(),
    })
}

#[tauri::command]
fn download_to_local_relative(url: String, relative_path: String, download_dir: Option<String>) -> Result<LocalDownloadResult, String> {
    let dir = download_dir
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(default_download_dir);
    let requested = clean_relative_path(&relative_path, "download.bin");
    let parent = requested.parent().map(Path::to_path_buf).unwrap_or_default();
    let filename = requested
        .file_name()
        .and_then(|value| value.to_str())
        .map(str::to_string)
        .unwrap_or_else(|| "download.bin".to_string());
    let target_dir = dir.join(parent);
    fs::create_dir_all(&target_dir).map_err(|error| format!("无法创建下载目录: {error}"))?;
    let path = unique_path(&target_dir, &filename);

    let status = Command::new("curl.exe")
        .args(["-L", "-f", "--retry", "2", "-o"])
        .arg(&path)
        .arg(&url)
        .status()
        .map_err(|error| format!("无法启动下载工具 curl.exe: {error}"))?;

    if !status.success() {
        let _ = fs::remove_file(&path);
        return Err("下载失败，请检查直链是否可访问".to_string());
    }

    Ok(LocalDownloadResult {
        path: path.display().to_string(),
    })
}

#[tauri::command]
fn reveal_in_folder(path: String) -> Result<(), String> {
    #[cfg(windows)]
    {
        let target = PathBuf::from(path);
        let folder = if target.is_dir() {
            target
        } else {
            target.parent().map(Path::to_path_buf).unwrap_or_else(default_download_dir)
        };
        let normalized = folder.display().to_string().replace('/', "\\");
        let status = Command::new("explorer.exe")
            .arg(&normalized)
            .status()
            .map_err(|error| format!("无法打开文件夹: {error}"))?;
        if !status.success() {
            return Err("打开文件夹失败".to_string());
        }
        return Ok(());
    }

    #[cfg(not(windows))]
    {
        let parent = Path::new(&path).parent().unwrap_or_else(|| Path::new("."));
        Command::new("xdg-open")
            .arg(parent)
            .spawn()
            .map_err(|error| format!("无法打开文件夹: {error}"))?;
        Ok(())
    }
}

#[cfg(windows)]
fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
fn credential_error(action: &str) -> String {
    format!("{action}: {}", std::io::Error::last_os_error())
}

#[cfg(windows)]
fn credential_not_found() -> bool {
    std::io::Error::last_os_error().raw_os_error() == Some(1168)
}

fn credential_target(instance_id: &str) -> String {
    let safe_id = instance_id
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
        .collect::<String>();
    format!(
        "{}{}",
        CREDENTIAL_TARGET_PREFIX,
        if safe_id.is_empty() { "default" } else { &safe_id }
    )
}

#[cfg(windows)]
#[tauri::command]
fn save_openlist_token(instance_id: String, token: String) -> Result<(), String> {
    use windows_sys::Win32::Security::Credentials::{
        CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
    };

    let token = token.trim();
    if token.is_empty() {
        return clear_openlist_token(instance_id);
    }

    let target_name = credential_target(&instance_id);
    let target = wide_null(&target_name);
    let user = wide_null(CREDENTIAL_USER);
    let mut blob = token.as_bytes().to_vec();
    let credential = CREDENTIALW {
        Type: CRED_TYPE_GENERIC,
        TargetName: target.as_ptr() as *mut u16,
        CredentialBlobSize: blob.len() as u32,
        CredentialBlob: blob.as_mut_ptr(),
        Persist: CRED_PERSIST_LOCAL_MACHINE,
        UserName: user.as_ptr() as *mut u16,
        ..Default::default()
    };

    let ok = unsafe { CredWriteW(&credential, 0) };
    if ok == 0 {
        return Err(credential_error("无法写入 Windows Credential Manager"));
    }

    Ok(())
}

#[cfg(not(windows))]
#[tauri::command]
fn save_openlist_token(_instance_id: String, _token: String) -> Result<(), String> {
    Err("当前平台暂未接入系统凭据存储".to_string())
}

#[cfg(windows)]
#[tauri::command]
fn read_openlist_token(instance_id: String) -> Result<Option<String>, String> {
    use std::slice;
    use windows_sys::Win32::Security::Credentials::{CredFree, CredReadW, CREDENTIALW, CRED_TYPE_GENERIC};

    let target_name = credential_target(&instance_id);
    let target = wide_null(&target_name);
    let mut credential: *mut CREDENTIALW = std::ptr::null_mut();
    let mut ok = unsafe { CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut credential) };
    if ok == 0 && credential_not_found() {
        let legacy_target = wide_null(LEGACY_CREDENTIAL_TARGET);
        ok = unsafe { CredReadW(legacy_target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut credential) };
    }
    if ok == 0 {
        if credential_not_found() {
            return Ok(None);
        }
        return Err(credential_error("无法读取 Windows Credential Manager"));
    }

    let token = unsafe {
        let credential_ref = &*credential;
        let bytes = slice::from_raw_parts(
            credential_ref.CredentialBlob,
            credential_ref.CredentialBlobSize as usize,
        );
        let token = String::from_utf8_lossy(bytes).trim().to_string();
        CredFree(credential as *const _);
        token
    };

    Ok((!token.is_empty()).then_some(token))
}

#[cfg(not(windows))]
#[tauri::command]
fn read_openlist_token(_instance_id: String) -> Result<Option<String>, String> {
    Ok(None)
}

#[cfg(windows)]
#[tauri::command]
fn clear_openlist_token(instance_id: String) -> Result<(), String> {
    use windows_sys::Win32::Security::Credentials::{CredDeleteW, CRED_TYPE_GENERIC};

    let target_name = credential_target(&instance_id);
    let target = wide_null(&target_name);
    let ok = unsafe { CredDeleteW(target.as_ptr(), CRED_TYPE_GENERIC, 0) };
    if ok == 0 && !credential_not_found() {
        return Err(credential_error("无法删除 Windows Credential Manager 凭据"));
    }

    Ok(())
}

#[cfg(not(windows))]
#[tauri::command]
fn clear_openlist_token(_instance_id: String) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
fn builtin_openlist_status(app: AppHandle) -> Result<BuiltinOpenListStatus, String> {
    let binary = openlist_binary(&app);
    let data_dir = app_data_dir(&app).ok();
    let running = is_port_open();

    Ok(BuiltinOpenListStatus {
        available: binary.is_some(),
        running,
        server_url: OPENLIST_URL.to_string(),
        binary_path: binary.as_ref().map(|path| path.display().to_string()),
        data_dir: data_dir.as_ref().map(|path| path.display().to_string()),
        message: if binary.is_some() {
            if running {
                "内置 OpenList 已可连接".to_string()
            } else {
                "内置 OpenList 已随应用提供，可启动".to_string()
            }
        } else {
            "安装包中未找到内置 OpenList".to_string()
        },
    })
}

#[tauri::command]
fn start_builtin_openlist(app: AppHandle) -> Result<BuiltinOpenListSession, String> {
    let binary = openlist_binary(&app).ok_or_else(|| "安装包中未找到内置 OpenList".to_string())?;
    let data_dir = app_data_dir(&app)?;

    if !is_port_open() {
        hidden_command(&binary)
            .args(["server", "--data"])
            .arg(&data_dir)
            .spawn()
            .map_err(|error| format!("无法启动内置 OpenList: {error}"))?;

        for _ in 0..40 {
            if is_port_open() {
                break;
            }
            thread::sleep(Duration::from_millis(250));
        }
    }

    if !is_port_open() {
        return Err("内置 OpenList 启动超时，请检查端口 5244 是否被占用".to_string());
    }

    let token = read_admin_token(&binary, &data_dir)?;
    let admin_password = ensure_builtin_admin_password(&binary, &data_dir)?;

    Ok(BuiltinOpenListSession {
        server_url: OPENLIST_URL.to_string(),
        token,
        data_dir: data_dir.display().to_string(),
        admin_username: "admin".to_string(),
        admin_password,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            builtin_openlist_status,
            start_builtin_openlist,
            save_openlist_token,
            read_openlist_token,
            clear_openlist_token,
            default_download_path,
            download_to_local,
            download_to_local_relative,
            reveal_in_folder
        ])
        .run(tauri::generate_context!())
        .expect("error while running OpenList Explorer");
}
