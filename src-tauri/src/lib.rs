use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io,
    fs,
    net::{TcpStream, ToSocketAddrs},
    path::{Path, PathBuf},
    process::Command,
    sync::{Mutex, OnceLock},
    thread,
    time::{Duration, Instant},
};
use bytes::Bytes;
use futures_util::stream;
use tauri::{AppHandle, Emitter, Manager, WindowEvent};
use rusqlite::{params, Connection};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, time::sleep};

const OPENLIST_BINS: [&str; 2] = ["openlist.exe", "openlist-x86_64-pc-windows-msvc.exe"];
const ARIA2_BINS: [&str; 2] = ["aria2c.exe", "aria2c-x86_64-pc-windows-msvc.exe"];
const MANAGED_OPENLIST_EXE: &str = "openlist_explorer_builtin.exe";
const MANAGED_ARIA2_EXE: &str = "aria2_explorer_builtin.exe";
const BUILTIN_OPENLIST_PORT: u16 = 15244;
const BUILTIN_OPENLIST_URL: &str = "http://127.0.0.1:15244";
const CREDENTIAL_TARGET_PREFIX: &str = "OpenList Explorer:OpenList API Token:";
const LEGACY_CREDENTIAL_TARGET: &str = "OpenList Explorer:OpenList API Token";
const CREDENTIAL_USER: &str = "OpenList Explorer";
const BUILTIN_ADMIN_PASSWORD_ID: &str = "builtin-local-admin-password";

fn percent_encode_header_path(path: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut encoded = String::with_capacity(path.len());
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~' | b'/') {
            encoded.push(byte as char);
        } else {
            encoded.push('%');
            encoded.push(HEX[(byte >> 4) as usize] as char);
            encoded.push(HEX[(byte & 0x0f) as usize] as char);
        }
    }
    encoded
}

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
    rpc_port: u16,
    download_dir: Option<String>,
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
struct UrlProbeResult {
    ok: bool,
    status: u16,
    content_type: Option<String>,
    content_length: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LocalUploadSelection {
    path: String,
    relative_path: String,
    size: u64,
}

#[derive(Deserialize, Serialize)]
struct FavoriteRow {
    id: String,
    storage: String,
    path: String,
}

#[derive(Deserialize, Serialize)]
struct HistoryRow {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    path: String,
    time: i64,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct TaskRow {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    status: String,
    progress: i64,
    speed: i64,
    path: String,
    local_path: Option<String>,
    remote_id: Option<String>,
    remote_url: Option<String>,
    instance_id: Option<String>,
    source: Option<String>,
    stage: Option<String>,
    raw_status: Option<String>,
    failure_reason: Option<String>,
    completed_dir: Option<String>,
    message: Option<String>,
    name: String,
    created_at: i64,
    updated_at: Option<i64>,
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
    is_local_port_open(BUILTIN_OPENLIST_PORT)
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
    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS favorites (
                id TEXT PRIMARY KEY NOT NULL,
                storage TEXT NOT NULL,
                path TEXT NOT NULL
            )",
            [],
        )
        .map_err(|error| format!("无法初始化 favorites 表: {error}"))?;
    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS history (
                id TEXT PRIMARY KEY NOT NULL,
                type TEXT NOT NULL,
                path TEXT NOT NULL,
                time INTEGER NOT NULL
            )",
            [],
        )
        .map_err(|error| format!("无法初始化 history 表: {error}"))?;
    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY NOT NULL,
                type TEXT NOT NULL,
                status TEXT NOT NULL,
                progress INTEGER NOT NULL,
                speed INTEGER NOT NULL,
                path TEXT NOT NULL,
                local_path TEXT,
                remote_id TEXT,
                remote_url TEXT,
                instance_id TEXT,
                source TEXT,
                stage TEXT,
                raw_status TEXT,
                failure_reason TEXT,
                completed_dir TEXT,
                message TEXT,
                name TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER
            )",
            [],
        )
        .map_err(|error| format!("无法初始化 tasks 表: {error}"))?;
    let _ = connection.execute("ALTER TABLE tasks ADD COLUMN remote_url TEXT", []);
    let _ = connection.execute("ALTER TABLE tasks ADD COLUMN instance_id TEXT", []);
    let _ = connection.execute("ALTER TABLE tasks ADD COLUMN stage TEXT", []);
    let _ = connection.execute("ALTER TABLE tasks ADD COLUMN raw_status TEXT", []);
    let _ = connection.execute("ALTER TABLE tasks ADD COLUMN failure_reason TEXT", []);
    let _ = connection.execute("ALTER TABLE tasks ADD COLUMN completed_dir TEXT", []);
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
        .or_else(find_aria2_in_path)
}

fn managed_sidecar_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("无法获取应用数据目录: {error}"))?
        .join("sidecars");
    fs::create_dir_all(&dir).map_err(|error| format!("无法创建内置组件目录: {error}"))?;
    Ok(dir)
}

fn managed_sidecar_binary(app: &AppHandle, source: &Path, managed_name: &str) -> Result<PathBuf, String> {
    let target = managed_sidecar_dir(app)?.join(managed_name);
    if !target.exists() {
        fs::copy(source, &target).map_err(|error| format!("无法释放内置组件 {managed_name}: {error}"))?;
    }
    Ok(target)
}

fn managed_openlist_binary(app: &AppHandle) -> Result<PathBuf, String> {
    let source = openlist_binary(app).ok_or_else(|| "安装包中未找到内置 OpenList".to_string())?;
    managed_sidecar_binary(app, &source, MANAGED_OPENLIST_EXE)
}

fn managed_aria2_binary(app: &AppHandle) -> Option<PathBuf> {
    let source = aria2_binary(app)?;
    if source.file_name().and_then(|name| name.to_str()) == Some(MANAGED_ARIA2_EXE) {
        return Some(source);
    }
    managed_sidecar_binary(app, &source, MANAGED_ARIA2_EXE).ok()
}

#[cfg(windows)]
fn stop_managed_process(image_name: &str) {
    let _ = hidden_program("taskkill.exe")
        .args(["/IM", image_name, "/F", "/T"])
        .output();
}

#[cfg(not(windows))]
fn stop_managed_process(_image_name: &str) {}

fn stop_managed_sidecars() {
    stop_managed_process(MANAGED_OPENLIST_EXE);
    stop_managed_process(MANAGED_ARIA2_EXE);
}

#[cfg(windows)]
fn find_aria2_in_path() -> Option<PathBuf> {
    let output = hidden_program("where.exe").arg("aria2c.exe").output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .find(|path| path.exists())
}

#[cfg(not(windows))]
fn find_aria2_in_path() -> Option<PathBuf> {
    let output = Command::new("which").arg("aria2c").output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
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

fn ensure_builtin_openlist_config(binary: &PathBuf, data_dir: &PathBuf) -> Result<(), String> {
    fs::create_dir_all(data_dir).map_err(|error| format!("无法创建内置 OpenList 数据目录: {error}"))?;
    let config_path = data_dir.join("config.json");

    if !config_path.exists() {
        let _ = hidden_command(binary)
            .args(["admin", "token", "--data"])
            .arg(data_dir)
            .output()
            .map_err(|error| format!("无法初始化内置 OpenList 配置: {error}"))?;
    }

    if !config_path.exists() {
        return Err("内置 OpenList 配置初始化失败，未生成 config.json".to_string());
    }

    let config_text = fs::read_to_string(&config_path)
        .map_err(|error| format!("无法读取内置 OpenList 配置: {error}"))?;
    let mut config: serde_json::Value = serde_json::from_str(&config_text)
        .map_err(|error| format!("无法解析内置 OpenList 配置: {error}"))?;
    let config_object = config
        .as_object_mut()
        .ok_or_else(|| "内置 OpenList 配置格式异常".to_string())?;
    let scheme = config_object
        .entry("scheme".to_string())
        .or_insert_with(|| serde_json::json!({}));
    if !scheme.is_object() {
        *scheme = serde_json::json!({});
    }
    let scheme_object = scheme
        .as_object_mut()
        .ok_or_else(|| "内置 OpenList 网络配置格式异常".to_string())?;
    scheme_object.insert("address".to_string(), serde_json::json!("127.0.0.1"));
    scheme_object.insert("http_port".to_string(), serde_json::json!(BUILTIN_OPENLIST_PORT));

    let next_text = serde_json::to_string_pretty(&config)
        .map_err(|error| format!("无法写入内置 OpenList 配置: {error}"))?;
    fs::write(&config_path, next_text)
        .map_err(|error| format!("无法保存内置 OpenList 配置: {error}"))?;
    Ok(())
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

fn reset_builtin_admin_password(binary: &PathBuf, data_dir: &PathBuf) -> Result<String, String> {
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

fn ensure_builtin_admin_password(binary: &PathBuf, data_dir: &PathBuf) -> Result<String, String> {
    if let Some(password) = read_openlist_token(BUILTIN_ADMIN_PASSWORD_ID.to_string())? {
        return Ok(password);
    }
    reset_builtin_admin_password(binary, data_dir)
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

fn part_path_for(path: &Path) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(".part");
    PathBuf::from(value)
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
fn db_get_favorites(app: AppHandle) -> Result<Vec<FavoriteRow>, String> {
    let connection = open_database(&app)?;
    let mut statement = connection
        .prepare("SELECT id, storage, path FROM favorites ORDER BY rowid DESC")
        .map_err(|error| format!("无法读取 favorites 表: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok(FavoriteRow {
                id: row.get(0)?,
                storage: row.get(1)?,
                path: row.get(2)?,
            })
        })
        .map_err(|error| format!("无法查询 favorites 表: {error}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("无法解析 favorites 表: {error}"))
}

#[tauri::command]
fn db_replace_favorites(app: AppHandle, items: Vec<FavoriteRow>) -> Result<(), String> {
    let mut connection = open_database(&app)?;
    let transaction = connection.transaction().map_err(|error| format!("无法写入 favorites 表: {error}"))?;
    transaction
        .execute("DELETE FROM favorites", [])
        .map_err(|error| format!("无法清空 favorites 表: {error}"))?;
    for item in items {
        transaction
            .execute(
                "INSERT INTO favorites(id, storage, path) VALUES(?1, ?2, ?3)",
                params![item.id, item.storage, item.path],
            )
            .map_err(|error| format!("无法写入 favorites 表: {error}"))?;
    }
    transaction.commit().map_err(|error| format!("无法提交 favorites 表: {error}"))?;
    Ok(())
}

#[tauri::command]
fn db_get_history(app: AppHandle) -> Result<Vec<HistoryRow>, String> {
    let connection = open_database(&app)?;
    let mut statement = connection
        .prepare("SELECT id, type, path, time FROM history ORDER BY time DESC LIMIT 100")
        .map_err(|error| format!("无法读取 history 表: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok(HistoryRow {
                id: row.get(0)?,
                kind: row.get(1)?,
                path: row.get(2)?,
                time: row.get(3)?,
            })
        })
        .map_err(|error| format!("无法查询 history 表: {error}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("无法解析 history 表: {error}"))
}

#[tauri::command]
fn db_replace_history(app: AppHandle, items: Vec<HistoryRow>) -> Result<(), String> {
    let mut connection = open_database(&app)?;
    let transaction = connection.transaction().map_err(|error| format!("无法写入 history 表: {error}"))?;
    transaction
        .execute("DELETE FROM history", [])
        .map_err(|error| format!("无法清空 history 表: {error}"))?;
    for item in items {
        transaction
            .execute(
                "INSERT INTO history(id, type, path, time) VALUES(?1, ?2, ?3, ?4)",
                params![item.id, item.kind, item.path, item.time],
            )
            .map_err(|error| format!("无法写入 history 表: {error}"))?;
    }
    transaction.commit().map_err(|error| format!("无法提交 history 表: {error}"))?;
    Ok(())
}

#[tauri::command]
fn db_get_tasks(app: AppHandle) -> Result<Vec<TaskRow>, String> {
    let connection = open_database(&app)?;
    let mut statement = connection
        .prepare(
            "SELECT id, type, status, progress, speed, path, local_path, remote_id, remote_url, instance_id, source, stage, raw_status, failure_reason, completed_dir, message, name, created_at, updated_at
             FROM tasks ORDER BY created_at DESC",
        )
        .map_err(|error| format!("无法读取 tasks 表: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok(TaskRow {
                id: row.get(0)?,
                kind: row.get(1)?,
                status: row.get(2)?,
                progress: row.get(3)?,
                speed: row.get(4)?,
                path: row.get(5)?,
                local_path: row.get(6)?,
                remote_id: row.get(7)?,
                remote_url: row.get(8)?,
                instance_id: row.get(9)?,
                source: row.get(10)?,
                stage: row.get(11)?,
                raw_status: row.get(12)?,
                failure_reason: row.get(13)?,
                completed_dir: row.get(14)?,
                message: row.get(15)?,
                name: row.get(16)?,
                created_at: row.get(17)?,
                updated_at: row.get(18)?,
            })
        })
        .map_err(|error| format!("无法查询 tasks 表: {error}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("无法解析 tasks 表: {error}"))
}

#[tauri::command]
fn db_replace_tasks(app: AppHandle, items: Vec<TaskRow>) -> Result<(), String> {
    let mut connection = open_database(&app)?;
    let transaction = connection.transaction().map_err(|error| format!("无法写入 tasks 表: {error}"))?;
    transaction
        .execute("DELETE FROM tasks", [])
        .map_err(|error| format!("无法清空 tasks 表: {error}"))?;
    for item in items {
        transaction
            .execute(
                "INSERT INTO tasks(id, type, status, progress, speed, path, local_path, remote_id, remote_url, instance_id, source, stage, raw_status, failure_reason, completed_dir, message, name, created_at, updated_at)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
                params![
                    item.id,
                    item.kind,
                    item.status,
                    item.progress,
                    item.speed,
                    item.path,
                    item.local_path,
                    item.remote_id,
                    item.remote_url,
                    item.instance_id,
                    item.source,
                    item.stage,
                    item.raw_status,
                    item.failure_reason,
                    item.completed_dir,
                    item.message,
                    item.name,
                    item.created_at,
                    item.updated_at
                ],
            )
            .map_err(|error| format!("无法写入 tasks 表: {error}"))?;
    }
    transaction.commit().map_err(|error| format!("无法提交 tasks 表: {error}"))?;
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
    let part_path = part_path_for(&path);
    let existing = tokio::fs::metadata(&part_path).await.map(|metadata| metadata.len()).unwrap_or(0);
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(existing > 0)
        .write(true)
        .truncate(existing == 0)
        .open(&part_path)
        .await
        .map_err(|error| format!("无法创建下载文件: {error}"))?;

    let client = reqwest::Client::new();
    let mut request = client.get(&url);
    if existing > 0 {
        request = request.header("Range", format!("bytes={existing}-"));
    }
    let response = request
        .send()
        .await
        .map_err(|error| format!("下载请求失败: {error}"))?
        .error_for_status()
        .map_err(|error| format!("下载地址不可用: {error}"))?;
    let accepts_resume = existing > 0 && response.status() == reqwest::StatusCode::PARTIAL_CONTENT;
    if existing > 0 && !accepts_resume {
        file.set_len(0)
            .await
            .map_err(|error| format!("无法重置断点文件: {error}"))?;
    }
    let offset = if accepts_resume { existing } else { 0 };
    let total = response.content_length().map(|length| length + offset).unwrap_or(0);
    let mut downloaded = offset;
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
    drop(file);
    tokio::fs::rename(&part_path, &path)
        .await
        .map_err(|error| format!("无法完成断点文件重命名: {error}"))?;
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

    emit_transfer(&app, &id, "running", 1, 0, None);
    let started = Instant::now();
    let filename = local
        .file_name()
        .and_then(|value| value.to_str())
        .map(str::to_string)
        .unwrap_or_else(|| "upload.bin".to_string());
    let file = tokio::fs::File::open(&local)
        .await
        .map_err(|error| format!("无法打开本地上传文件: {error}"))?;
    let stream_app = app.clone();
    let stream_id = id.clone();
    let upload_stream = stream::unfold((file, 0_u64, started), move |(mut file, mut uploaded, started)| {
        let app = stream_app.clone();
        let id = stream_id.clone();
        async move {
            loop {
                let control = match transfer_control_snapshot(&id) {
                    Ok(value) => value,
                    Err(error) => {
                        return Some((Err(io::Error::new(io::ErrorKind::Other, error)), (file, uploaded, started)));
                    }
                };
                if control.canceled {
                    emit_transfer(&app, &id, "canceled", 0, 0, None);
                    return Some((
                        Err(io::Error::new(io::ErrorKind::Interrupted, "上传已取消")),
                        (file, uploaded, started),
                    ));
                }
                if !control.paused {
                    break;
                }
                let progress = if size > 0 {
                    ((uploaded.saturating_mul(100) / size).min(99)) as u8
                } else {
                    0
                };
                emit_transfer(&app, &id, "paused", progress, 0, None);
                sleep(Duration::from_millis(250)).await;
            }

            let mut buffer = vec![0_u8; 256 * 1024];
            match file.read(&mut buffer).await {
                Ok(0) => None,
                Ok(read) => {
                    buffer.truncate(read);
                    uploaded += read as u64;
                    let elapsed = started.elapsed().as_secs().max(1);
                    let progress = if size > 0 {
                        ((uploaded.saturating_mul(100) / size).min(99)) as u8
                    } else {
                        0
                    };
                    emit_transfer(&app, &id, "running", progress, uploaded / elapsed, None);
                    Some((Ok::<Bytes, io::Error>(Bytes::from(buffer)), (file, uploaded, started)))
                }
                Err(error) => Some((Err(error), (file, uploaded, started))),
            }
        }
    });
    let body = reqwest::Body::wrap_stream(upload_stream);
    let part = reqwest::multipart::Part::stream_with_length(body, size).file_name(filename);
    let form = reqwest::multipart::Form::new().part("file", part);
    let base_url = server_url.trim().trim_end_matches('/');
    let target_url = format!("{base_url}/api/fs/form");

    let response = reqwest::Client::new()
        .put(target_url)
        .header("Authorization", token.trim())
        .header("File-Path", percent_encode_header_path(&remote_path))
        .multipart(form)
        .send()
        .await
        .map_err(|error| {
            if transfer_control_snapshot(&id).map(|control| control.canceled).unwrap_or(false) {
                "上传已取消".to_string()
            } else {
                format!("上传请求失败: {error}")
            }
        })?;
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

#[tauri::command]
async fn probe_url(url: String) -> Result<UrlProbeResult, String> {
    let client = reqwest::Client::new();
    let mut response = client
        .head(&url)
        .send()
        .await
        .map_err(|error| format!("直链探测失败: {error}"))?;

    if response.status() == reqwest::StatusCode::METHOD_NOT_ALLOWED {
        response = client
            .get(&url)
            .header("Range", "bytes=0-0")
            .send()
            .await
            .map_err(|error| format!("直链探测失败: {error}"))?;
    }

    let status = response.status();
    let headers = response.headers();
    let content_type = headers
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let content_length = headers
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());

    Ok(UrlProbeResult {
        ok: status.is_success(),
        status: status.as_u16(),
        content_type,
        content_length,
    })
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
        server_url: BUILTIN_OPENLIST_URL.to_string(),
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
fn local_aria2_status(app: AppHandle, rpc_port: Option<u16>) -> Result<LocalAria2Status, String> {
    let binary = managed_aria2_binary(&app).or_else(|| aria2_binary(&app));
    let port = rpc_port.unwrap_or(6800);
    let running = is_local_port_open(port);

    Ok(LocalAria2Status {
        available: binary.is_some(),
        running,
        rpc_url: format!("http://127.0.0.1:{port}/jsonrpc"),
        rpc_port: port,
        download_dir: Some(default_download_dir().display().to_string()),
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
fn start_local_aria2(
    app: AppHandle,
    rpc_port: u16,
    rpc_secret: Option<String>,
    download_dir: Option<String>,
    max_concurrent: Option<u16>,
    split: Option<u16>,
) -> Result<LocalAria2Status, String> {
    let binary = managed_aria2_binary(&app).ok_or_else(|| {
        "未找到 aria2c.exe。请把 aria2c.exe 放到安装目录、安装目录的 binaries 子目录，或加入系统 PATH。".to_string()
    })?;
    let port = if rpc_port == 0 { 6800 } else { rpc_port };
    let max_concurrent = max_concurrent.unwrap_or(5).clamp(1, 32);
    let split = split.unwrap_or(8).clamp(1, 32);
    let download_dir = download_dir
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(default_download_dir);

    if !is_local_port_open(port) {
        let aria2_dir = app
            .path()
            .app_data_dir()
            .map_err(|error| format!("无法获取应用数据目录: {error}"))?
            .join("aria2");
        fs::create_dir_all(&aria2_dir).map_err(|error| format!("无法创建 Aria2 数据目录: {error}"))?;
        let session_path = aria2_dir.join("aria2.session");
        if !session_path.exists() {
            fs::write(&session_path, "").map_err(|error| format!("无法创建 Aria2 session 文件: {error}"))?;
        }

        let mut command = hidden_command(&binary);
        command.args([
            "--enable-rpc=true",
            "--rpc-listen-all=false",
            "--continue=true",
            "--auto-file-renaming=true",
            "--save-session-interval=30",
        ]);
        command.arg(format!("--rpc-listen-port={port}"));
        command.arg(format!("--max-concurrent-downloads={max_concurrent}"));
        command.arg(format!("--split={split}"));
        command.arg(format!("--max-connection-per-server={split}"));
        command.arg(format!("--dir={}", download_dir.display()));
        command.arg(format!("--input-file={}", session_path.display()));
        command.arg(format!("--save-session={}", session_path.display()));
        if let Some(secret) = rpc_secret.map(|value| value.trim().to_string()).filter(|value| !value.is_empty()) {
            command.arg(format!("--rpc-secret={secret}"));
        }
        command.spawn().map_err(|error| format!("无法启动 Aria2: {error}"))?;

        for _ in 0..20 {
            if is_local_port_open(port) {
                break;
            }
            thread::sleep(Duration::from_millis(250));
        }
    }

    let mut status = local_aria2_status(app, Some(port))?;
    status.download_dir = Some(download_dir.display().to_string());
    Ok(status)
}

#[tauri::command]
fn start_builtin_openlist(app: AppHandle) -> Result<BuiltinOpenListSession, String> {
    let binary = managed_openlist_binary(&app)?;
    let data_dir = app_data_dir(&app)?;
    ensure_builtin_openlist_config(&binary, &data_dir)?;

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
        return Err(format!("内置 OpenList 启动超时，请检查端口 {BUILTIN_OPENLIST_PORT} 是否被占用"));
    }

    let token = read_admin_token(&binary, &data_dir)?;
    let admin_password = ensure_builtin_admin_password(&binary, &data_dir)?;

    Ok(BuiltinOpenListSession {
        server_url: BUILTIN_OPENLIST_URL.to_string(),
        token,
        data_dir: data_dir.display().to_string(),
        admin_username: "admin".to_string(),
        admin_password,
    })
}

#[tauri::command]
fn reset_builtin_openlist_admin_password(app: AppHandle) -> Result<String, String> {
    let binary = managed_openlist_binary(&app)?;
    let data_dir = app_data_dir(&app)?;
    ensure_builtin_openlist_config(&binary, &data_dir)?;
    reset_builtin_admin_password(&binary, &data_dir)
}

#[tauri::command]
fn open_external_url(url: String) -> Result<(), String> {
    let trimmed = url.trim();
    if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
        return Err("只能打开 http:// 或 https:// 地址".to_string());
    }

    #[cfg(windows)]
    {
        let status = hidden_program("cmd.exe")
            .args(["/C", "start", "", trimmed])
            .status()
            .map_err(|error| format!("无法打开系统浏览器: {error}"))?;
        if !status.success() {
            return Err("系统浏览器打开失败".to_string());
        }
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(trimmed)
            .spawn()
            .map_err(|error| format!("无法打开系统浏览器: {error}"))?;
        return Ok(());
    }

    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(trimmed)
            .spawn()
            .map_err(|error| format!("无法打开系统浏览器: {error}"))?;
        Ok(())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .on_window_event(|_window, event| {
            if matches!(event, WindowEvent::CloseRequested { .. }) {
                stop_managed_sidecars();
            }
        })
        .invoke_handler(tauri::generate_handler![
            builtin_openlist_status,
            local_aria2_status,
            start_local_aria2,
            start_builtin_openlist,
            reset_builtin_openlist_admin_password,
            open_external_url,
            save_openlist_token,
            read_openlist_token,
            clear_openlist_token,
            select_upload_files,
            expand_upload_paths,
            db_get_json,
            db_set_json,
            db_get_favorites,
            db_replace_favorites,
            db_get_history,
            db_replace_history,
            db_get_tasks,
            db_replace_tasks,
            default_download_path,
            download_to_local,
            download_to_local_relative,
            download_with_engine,
            upload_with_engine,
            pause_transfer_task,
            resume_transfer_task,
            cancel_transfer_task,
            reveal_in_folder,
            probe_url
        ])
        .run(tauri::generate_context!())
        .expect("error while running OpenList Explorer");
}
