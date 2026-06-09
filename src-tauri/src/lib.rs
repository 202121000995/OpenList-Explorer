use serde::Serialize;
use std::{
    collections::HashMap,
    fs,
    net::{TcpStream, ToSocketAddrs},
    path::{Path, PathBuf},
    process::Command,
    sync::{Mutex, OnceLock},
    thread,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager};
use rusqlite::{params, Connection};
use tokio::{io::AsyncWriteExt, time::sleep};

const OPENLIST_BINS: [&str; 2] = ["openlist.exe", "openlist-x86_64-pc-windows-msvc.exe"];
const ARIA2_BINS: [&str; 2] = ["aria2c.exe", "aria2c-x86_64-pc-windows-msvc.exe"];
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
struct LocalAria2Status {
    available: bool,
    running: bool,
    rpc_url: String,
    binary_path: Option<String>,
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LocalUploadSelection {
    path: String,
    relative_path: String,
    size: u64,
}

#[derive(Default, Clone)]
struct TransferControl {
    paused: bool,
    canceled: bool,
}

#[derive(Serialize, Clone)]
struct TransferProgress {
    id: String,
    status: String,
    progress: u8,
    speed: u64,
    local_path: Option<String>,
}

static TRANSFERS: OnceLock<Mutex<HashMap<String, TransferControl>>> = OnceLock::new();

fn transfers() -> &'static Mutex<HashMap<String, TransferControl>> {
    TRANSFERS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn is_port_open() -> bool {
    is_local_port_open(5244)
}

fn is_local_port_open(port: u16) -> bool {
    ("127.0.0.1", port)
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

fn database_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("无法获取应用数据目录: {error}"))?;
    fs::create_dir_all(&dir).map_err(|error| format!("无法创建应用数据目录: {error}"))?;
    Ok(dir.join("explorer.sqlite"))
}

fn open_database(app: &AppHandle) -> Result<Connection, String> {
    let connection = Connection::open(database_path(app)?).map_err(|error| format!("无法打开 SQLite: {error}"))?;
    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT NOT NULL
            )",
            [],
        )
        .map_err(|error| format!("无法初始化 SQLite: {error}"))?;
    Ok(connection)
}

fn candidate_binaries(app: &AppHandle) -> Vec<PathBuf> {
    candidate_named_binaries(app, &OPENLIST_BINS)
}

fn candidate_named_binaries(app: &AppHandle, names: &[&str]) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(resource_dir) = app.path().resource_dir() {
        for binary_name in names {
            paths.push(resource_dir.join(binary_name));
            paths.push(resource_dir.join("binaries").join(binary_name));
        }
    }

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            for binary_name in names {
                paths.push(parent.join(binary_name));
                paths.push(parent.join("binaries").join(binary_name));
            }
        }
    }

    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        for binary_name in names {
            paths.push(PathBuf::from(&manifest_dir).join("binaries").join(binary_name));
        }
    }

    paths
}

fn openlist_binary(app: &AppHandle) -> Option<PathBuf> {
    candidate_binaries(app).into_iter().find(|path| path.exists())
}

fn aria2_binary(app: &AppHandle) -> Option<PathBuf> {
    candidate_named_binaries(app, &ARIA2_BINS)
        .into_iter()
        .find(|path| path.exists())
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

fn hidden_program(program: &str) -> Command {
    let mut command = Command::new(program);
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

fn relative_upload_path(path: &Path, base: Option<&Path>) -> String {
    let relative = base
        .and_then(|base| path.strip_prefix(base).ok())
        .unwrap_or(path.file_name().map(Path::new).unwrap_or(path));
    let parts = relative
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .map(clean_filename)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.is_empty() {
        clean_filename(path.file_name().and_then(|value| value.to_str()).unwrap_or("upload.bin"))
    } else {
        parts.join("/")
    }
}

fn collect_upload_path(path: &Path, base: Option<&Path>, output: &mut Vec<LocalUploadSelection>) -> Result<(), String> {
    if path.is_file() {
        let metadata = fs::metadata(path).map_err(|error| format!("无法读取本地文件: {error}"))?;
        output.push(LocalUploadSelection {
            path: path.display().to_string(),
            relative_path: relative_upload_path(path, base),
            size: metadata.len(),
        });
        return Ok(());
    }

    if path.is_dir() {
        let dir_base = base.or_else(|| path.parent()).unwrap_or(path);
        for entry in fs::read_dir(path).map_err(|error| format!("无法读取本地目录: {error}"))? {
            let entry = entry.map_err(|error| format!("无法读取本地目录项: {error}"))?;
            collect_upload_path(&entry.path(), Some(dir_base), output)?;
        }
    }

    Ok(())
}

fn expand_upload_pathbufs(paths: Vec<PathBuf>) -> Result<Vec<LocalUploadSelection>, String> {
    let mut output = Vec::new();
    for path in paths {
        collect_upload_path(&path, None, &mut output)?;
    }
    Ok(output)
}

fn emit_transfer(app: &AppHandle, id: &str, status: &str, progress: u8, speed: u64, local_path: Option<String>) {
    let _ = app.emit(
        "transfer://progress",
        TransferProgress {
            id: id.to_string(),
            status: status.to_string(),
            progress,
            speed,
            local_path,
        },
    );
}

#[tauri::command]
fn default_download_path() -> String {
    default_download_dir().display().to_string()
}

#[tauri::command]
fn select_upload_files(pick_directory: bool) -> Result<Vec<LocalUploadSelection>, String> {
    #[cfg(windows)]
    {
        let script = if pick_directory {
            r#"
            [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
            Add-Type -AssemblyName System.Windows.Forms
            $dialog = New-Object System.Windows.Forms.FolderBrowserDialog
            $dialog.Description = '选择要上传的目录'
            if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
              @($dialog.SelectedPath) | ConvertTo-Json -Compress
            } else {
              '[]'
            }
            "#
        } else {
            r#"
            [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
            Add-Type -AssemblyName System.Windows.Forms
            $dialog = New-Object System.Windows.Forms.OpenFileDialog
            $dialog.Multiselect = $true
            $dialog.CheckFileExists = $true
            if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
              @($dialog.FileNames) | ConvertTo-Json -Compress
            } else {
              '[]'
            }
            "#
        };

        let output = hidden_program("powershell.exe")
            .args(["-NoProfile", "-STA", "-ExecutionPolicy", "Bypass", "-Command", script])
            .output()
            .map_err(|error| format!("无法打开文件选择器: {error}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(if stderr.is_empty() {
                "文件选择器打开失败".to_string()
            } else {
                stderr
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let selected: Vec<String> = serde_json::from_str(stdout.trim()).unwrap_or_default();
        return expand_upload_pathbufs(selected.into_iter().map(PathBuf::from).collect());
    }

    #[cfg(not(windows))]
    {
        let _ = pick_directory;
        Err("当前平台暂未实现系统文件选择器".to_string())
    }
}

#[tauri::command]
fn expand_upload_paths(paths: Vec<String>) -> Result<Vec<LocalUploadSelection>, String> {
    expand_upload_pathbufs(paths.into_iter().map(PathBuf::from).collect())
}

#[tauri::command]
fn db_get_json(app: AppHandle, key: String) -> Result<Option<String>, String> {
    let connection = open_database(&app)?;
    let mut statement = connection
        .prepare("SELECT value FROM settings WHERE key = ?1")
        .map_err(|error| format!("无法读取 SQLite: {error}"))?;
    let mut rows = statement
        .query(params![key])
        .map_err(|error| format!("无法查询 SQLite: {error}"))?;
    if let Some(row) = rows.next().map_err(|error| format!("无法读取 SQLite 行: {error}"))? {
        let value: String = row.get(0).map_err(|error| format!("无法解析 SQLite 值: {error}"))?;
        Ok(Some(value))
    } else {
        Ok(None)
    }
}

#[tauri::command]
fn db_set_json(app: AppHandle, key: String, value: String) -> Result<(), String> {
    let connection = open_database(&app)?;
    connection
        .execute(
            "INSERT INTO settings(key, value) VALUES(?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )
        .map_err(|error| format!("无法写入 SQLite: {error}"))?;
    Ok(())
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

fn set_transfer_control(id: &str, update: impl FnOnce(&mut TransferControl)) -> Result<(), String> {
    let mut transfers = transfers().lock().map_err(|_| "无法锁定传输状态".to_string())?;
    let control = transfers.entry(id.to_string()).or_default();
    update(control);
    Ok(())
}

fn transfer_control_snapshot(id: &str) -> Result<TransferControl, String> {
    let transfers = transfers().lock().map_err(|_| "无法读取传输状态".to_string())?;
    let control = transfers.get(id).cloned().unwrap_or_default();
    Ok(control)
}

#[tauri::command]
fn pause_transfer_task(id: String) -> Result<(), String> {
    set_transfer_control(&id, |control| control.paused = true)
}

#[tauri::command]
fn resume_transfer_task(id: String) -> Result<(), String> {
    set_transfer_control(&id, |control| control.paused = false)
}

#[tauri::command]
fn cancel_transfer_task(id: String) -> Result<(), String> {
    set_transfer_control(&id, |control| control.canceled = true)
}

#[tauri::command]
async fn download_with_engine(
    app: AppHandle,
    id: String,
    url: String,
    filename: String,
    relative_path: Option<String>,
    download_dir: Option<String>,
) -> Result<LocalDownloadResult, String> {
    set_transfer_control(&id, |control| {
        control.paused = false;
        control.canceled = false;
    })?;

    let dir = download_dir
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(default_download_dir);
    let requested = relative_path
        .as_deref()
        .map(|path| clean_relative_path(path, &filename))
        .unwrap_or_else(|| PathBuf::from(clean_filename(&filename)));
    let parent = requested.parent().map(Path::to_path_buf).unwrap_or_default();
    let file_name = requested
        .file_name()
        .and_then(|value| value.to_str())
        .map(str::to_string)
        .unwrap_or_else(|| clean_filename(&filename));
    let target_dir = dir.join(parent);
    tokio::fs::create_dir_all(&target_dir)
        .await
        .map_err(|error| format!("无法创建下载目录: {error}"))?;
    let path = unique_path(&target_dir, &file_name);
    let mut file = tokio::fs::File::create(&path)
        .await
        .map_err(|error| format!("无法创建下载文件: {error}"))?;

    let response = reqwest::get(&url)
        .await
        .map_err(|error| format!("下载请求失败: {error}"))?
        .error_for_status()
        .map_err(|error| format!("下载地址不可用: {error}"))?;
    let total = response.content_length().unwrap_or(0);
    let mut downloaded = 0_u64;
    let started = Instant::now();
    let mut response = response;

    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| format!("读取下载数据失败: {error}"))?
    {
        loop {
            let control = transfer_control_snapshot(&id)?;
            if control.canceled {
                let _ = tokio::fs::remove_file(&path).await;
                let _ = app.emit(
                    "transfer://progress",
                    TransferProgress {
                        id: id.clone(),
                        status: "canceled".to_string(),
                        progress: 0,
                        speed: 0,
                        local_path: None,
                    },
                );
                return Err("下载已取消".to_string());
            }
            if !control.paused {
                break;
            }
            let _ = app.emit(
                "transfer://progress",
                TransferProgress {
                    id: id.clone(),
                    status: "paused".to_string(),
                    progress: if total > 0 { ((downloaded * 100) / total) as u8 } else { 0 },
                    speed: 0,
                    local_path: None,
                },
            );
            sleep(Duration::from_millis(250)).await;
        }

        file.write_all(&chunk)
            .await
            .map_err(|error| format!("写入下载文件失败: {error}"))?;
        downloaded += chunk.len() as u64;
        let elapsed = started.elapsed().as_secs().max(1);
        let progress = if total > 0 {
            ((downloaded.saturating_mul(100) / total).min(99)) as u8
        } else {
            0
        };
        let _ = app.emit(
            "transfer://progress",
            TransferProgress {
                id: id.clone(),
                status: "running".to_string(),
                progress,
                speed: downloaded / elapsed,
                local_path: None,
            },
        );
    }

    file.flush()
        .await
        .map_err(|error| format!("保存下载文件失败: {error}"))?;
    let local_path = path.display().to_string();
    let _ = app.emit(
        "transfer://progress",
        TransferProgress {
            id: id.clone(),
            status: "success".to_string(),
            progress: 100,
            speed: 0,
            local_path: Some(local_path.clone()),
        },
    );
    if let Ok(mut transfers) = transfers().lock() {
        transfers.remove(&id);
    }

    Ok(LocalDownloadResult { path: local_path })
}

#[tauri::command]
async fn upload_with_engine(
    app: AppHandle,
    id: String,
    server_url: String,
    token: String,
    local_path: String,
    remote_path: String,
) -> Result<(), String> {
    set_transfer_control(&id, |control| {
        control.paused = false;
        control.canceled = false;
    })?;

    let local = PathBuf::from(local_path);
    if !local.is_file() {
        return Err("本地上传文件不存在".to_string());
    }
    let metadata = tokio::fs::metadata(&local)
        .await
        .map_err(|error| format!("无法读取本地上传文件: {error}"))?;
    let size = metadata.len();

    loop {
        let control = transfer_control_snapshot(&id)?;
        if control.canceled {
            emit_transfer(&app, &id, "canceled", 0, 0, None);
            return Err("上传已取消".to_string());
        }
        if !control.paused {
            break;
        }
        emit_transfer(&app, &id, "paused", 0, 0, None);
        sleep(Duration::from_millis(250)).await;
    }

    emit_transfer(&app, &id, "running", 5, 0, None);
    let started = Instant::now();
    let filename = local
        .file_name()
        .and_then(|value| value.to_str())
        .map(str::to_string)
        .unwrap_or_else(|| "upload.bin".to_string());
    let part = reqwest::multipart::Part::file(&local)
        .await
        .map_err(|error| format!("无法读取本地上传文件: {error}"))?
        .file_name(filename);
    let form = reqwest::multipart::Form::new().part("file", part);
    let base_url = server_url.trim().trim_end_matches('/');
    let target_url = format!("{base_url}/api/fs/form");

    emit_transfer(&app, &id, "running", 20, 0, None);
    let response = reqwest::Client::new()
        .put(target_url)
        .header("Authorization", token.trim())
        .header("File-Path", remote_path)
        .multipart(form)
        .send()
        .await
        .map_err(|error| format!("上传请求失败: {error}"))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|error| format!("读取上传响应失败: {error}"))?;

    if !status.is_success() {
        emit_transfer(&app, &id, "failed", 0, 0, None);
        return Err(format!("上传失败: HTTP {status} {text}"));
    }

    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
        let code = value.get("code").and_then(|item| item.as_i64()).unwrap_or(200);
        if code != 200 {
            let message = value
                .get("message")
                .or_else(|| value.get("msg"))
                .and_then(|item| item.as_str())
                .unwrap_or("OpenList 上传失败");
            emit_transfer(&app, &id, "failed", 0, 0, None);
            return Err(message.to_string());
        }
    }

    let elapsed = started.elapsed().as_secs().max(1);
    emit_transfer(&app, &id, "success", 100, size / elapsed, Some(local.display().to_string()));
    if let Ok(mut transfers) = transfers().lock() {
        transfers.remove(&id);
    }
    Ok(())
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
fn local_aria2_status(app: AppHandle) -> Result<LocalAria2Status, String> {
    let binary = aria2_binary(&app);
    let running = is_local_port_open(6800);

    Ok(LocalAria2Status {
        available: binary.is_some(),
        running,
        rpc_url: "http://127.0.0.1:6800/jsonrpc".to_string(),
        binary_path: binary.as_ref().map(|path| path.display().to_string()),
        message: if binary.is_some() {
            if running {
                "本机 Aria2 RPC 已可连接".to_string()
            } else {
                "安装包中已包含 Aria2，但尚未启动 RPC。".to_string()
            }
        } else {
            "安装包中未包含 aria2c.exe；云下载会使用 OpenList 已配置的下载工具。".to_string()
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
            local_aria2_status,
            start_builtin_openlist,
            save_openlist_token,
            read_openlist_token,
            clear_openlist_token,
            select_upload_files,
            expand_upload_paths,
            db_get_json,
            db_set_json,
            default_download_path,
            download_to_local,
            download_to_local_relative,
            download_with_engine,
            upload_with_engine,
            pause_transfer_task,
            resume_transfer_task,
            cancel_transfer_task,
            reveal_in_folder
        ])
        .run(tauri::generate_context!())
        .expect("error while running OpenList Explorer");
}
