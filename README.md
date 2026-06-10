# OpenList Explorer

OpenList Explorer 是基于 OpenList REST API 的桌面端统一存储资源管理器。OpenList 负责网盘协议、对象存储驱动和存储抽象，Explorer 负责文件管理、任务队列和桌面交互体验。

## 当前能力

- 内置 OpenList：首次启动可选择使用随软件释放的 OpenList，并自动启动、连接和读取管理信息。
- 连接已有 OpenList：支持保存多个 OpenList 实例，按实例切换地址、用户名、token、公网直链地址和默认实例。
- 凭据安全：Windows 版 token 写入 Windows Credential Manager，不写入 SQLite 或浏览器 localStorage。
- 文件管理：浏览、刷新、搜索、列表/网格视图、上传、下载、删除、重命名、新建目录、复制、移动、收藏、历史记录、复制路径、复制直链。
- 直链替换：仅当 OpenList 返回的直链是 localhost、127.0.0.1、内网 IP 或本地域名时，才使用用户设置的公网地址替换域名。
- 传输任务：Rust 后端托管本地上传/下载队列，支持进度、速度、暂停、继续、取消；下载使用 `.part` 文件支持断点续传。
- 云端下载：支持调用 OpenList 离线下载任务，并同步任务状态；已预留 Aria2 检测和本地 RPC 启动能力。
- 本地数据：设置、收藏、历史、任务落到 SQLite 结构化表，浏览器预览环境回退到 localStorage。

## 技术栈

- Tauri 2.x
- Vue 3
- TypeScript
- Pinia
- Vue Router
- Naive UI
- VueUse
- Axios
- SQLite / Drizzle schema
- Rust sidecar commands

## 开发

```powershell
npm install
npm run dev
```

桌面调试：

```powershell
npm run tauri:dev
```

类型检查：

```powershell
npm run typecheck
```

打包：

```powershell
npm run tauri:build
```

## Aria2 打包

默认源码不包含 `aria2c.exe`。需要把 Aria2 随安装包发布时，先执行：

```powershell
.\scripts\prepare_aria2_bundle.cmd
```

如果网络下载不方便，可以手动下载 Windows x64 版 Aria2 zip 后执行：

```powershell
.\scripts\prepare_aria2_bundle.cmd --zip D:\path\aria2.zip
```

脚本会复制 `aria2c.exe` 到 `src-tauri\binaries` 并更新 Tauri `externalBin`，之后重新运行 `npm run tauri:build`。

## OpenList API Smoke Test

需要已有 OpenList token：

```powershell
$env:OPENLIST_URL="http://127.0.0.1:5244"
$env:OPENLIST_TOKEN="你的 token"
$env:OPENLIST_E2E_PARENT="/某个测试挂载点"
.\scripts\e2e_openlist_smoke.cmd
```

测试会执行 mkdir、upload、list、rename、search、get raw、download probe、delete。请使用可以安全创建和删除文件的测试目录。

如果要从本机 OpenList 可执行文件读取 admin token，可以使用：

```powershell
$env:OPENLIST_URL="http://127.0.0.1:5244"
$env:OPENLIST_BIN="D:\wenjian\openlist-windows-amd64\openlist.exe"
$env:OPENLIST_FORCE_BIN_DIR="1"
.\scripts\e2e_openlist_smoke.cmd
```

脚本不会打印 token。未指定 `OPENLIST_E2E_PARENT` 时，会自动尝试根目录下的挂载点，找到可写目录后运行测试。

## P0 Release Check

发布前可以执行：

```powershell
.\scripts\p0_release_check.cmd
```

它会检查版本一致性、OpenList/Aria2 sidecar、安装包是否存在、sidecar 是否可执行、内置 OpenList 首次初始化是否能生成管理信息。设置 `OPENLIST_TOKEN` 或 `OPENLIST_BIN` 后，还会自动运行真实 OpenList API smoke test。

## Upload Resume Probe

字节级上传断点续传取决于 OpenList `/api/fs/form` 是否支持分片组装。可以执行：

```powershell
$env:OPENLIST_URL="http://127.0.0.1:5244"
$env:OPENLIST_BIN="D:\wenjian\openlist-windows-amd64\openlist.exe"
$env:OPENLIST_FORCE_BIN_DIR="1"
.\scripts\check_openlist_upload_resume.cmd
```

当前实测结果：OpenList 未按 `Content-Range` 组装两段上传内容，因此 Explorer 只能提供上传暂停/取消/失败后重新上传，不能标称为字节级断点续传。

## 发布

发布脚本通过 GitHub CLI 和 GitHub API 上传源码与安装包：

```powershell
.\scripts\publish_github.cmd
```

发布前需要：

- `gh auth status` 已登录目标账号。
- 已完成 `npm run tauri:build`。
- 版本号已递增。

脚本会排除 `node_modules`、`dist`、Tauri `target`、大体积二进制、安装包，以及和其他无关分析对话相关的路径关键词。
