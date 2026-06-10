# OpenList Explorer

OpenList Explorer 是基于 OpenList REST API 的桌面端统一存储资源管理器。OpenList 负责网盘协议、对象存储驱动和存储抽象，Explorer 负责文件管理、任务队列和桌面交互体验。

## 当前能力

- 内置 OpenList：首次启动可选择使用随软件释放的 OpenList，默认监听 `127.0.0.1:15244`，并自动启动、连接和读取管理信息。
- 连接已有 OpenList：支持保存多个 OpenList 实例，按实例切换地址、用户名、token、公网直链地址和默认实例。
- 凭据安全：Windows 版 token 写入 Windows Credential Manager，不写入 SQLite 或浏览器 localStorage。
- 文件管理：浏览、刷新、搜索、列表/网格视图、上传、下载、删除、重命名、新建目录、复制、移动、收藏、历史记录、复制路径、复制直链。
- 直链替换：仅当 OpenList 返回的直链是 localhost、127.0.0.1、内网 IP 或本地域名时，才使用用户设置的公网地址替换域名。
- 传输任务：Rust 后端托管本地上传/下载队列，支持进度、速度、暂停、继续、取消；下载使用 `.part` 文件支持断点续传。
- 云端下载：支持调用 OpenList 离线下载任务，并同步任务状态；安装包随带 Aria2，可一键启动本机 RPC，再到 OpenList 管理端启用 Aria2 工具。
- 本地数据：设置、收藏、历史、任务落到 SQLite。设置项按 `settings.*` 拆分保存，同时兼容旧版整体 `settings` 快照；浏览器预览环境回退到 localStorage。

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

响应式截图巡检：

```powershell
npm run build
npm run check:layout
```

脚本会使用本机 Chrome/Edge headless 打开 `dist`，对 `/files`、`/uploads`、`/downloads`、`/settings`、`/openlist` 在 760、1024、1280 宽度下截图。截图输出到 `src-tauri\target\responsive-layout`，该目录不会发布到 GitHub。

打包：

```powershell
npm run tauri:build
```

## Aria2 打包和使用

当前安装包已包含 `aria2c.exe`，设置页的“云下载 / Aria2”会拆分展示两层状态：

- 本机 Aria2 RPC：Explorer 随安装包启动的本机 Aria2 服务。
- OpenList 云下载工具：OpenList 管理端里已经启用的离线下载工具。

一键连接 Aria2 会启动本机 RPC，并直接使用软件“下载目录”设置。要让云端下载任务真正使用 Aria2，还需要在 OpenList 管理端把 Aria2 工具指向这个 RPC 地址。软件会提供复制配置和打开管理端入口。

如果重新准备 Aria2 sidecar，可以执行：

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

测试会执行 mkdir、upload、list、rename、copy、move、delete、search、get raw、download probe，并在 OpenList 启用离线下载工具时尝试云下载提交和状态读取。请使用可以安全创建和删除文件的测试目录。

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

它会检查版本一致性、OpenList/Aria2 sidecar、安装包是否存在、sidecar 是否可执行、内置 OpenList 首次初始化是否能生成管理信息、多 OpenList 实例隔离逻辑。设置 `OPENLIST_TOKEN` 或 `OPENLIST_BIN` 后，还会自动运行真实 OpenList API smoke test 和上传断点续传探测。

## Upload Resume Probe

字节级上传断点续传取决于 OpenList `/api/fs/form` 和具体存储驱动是否支持分片组装。可以执行：

```powershell
$env:OPENLIST_URL="http://127.0.0.1:5244"
$env:OPENLIST_BIN="D:\wenjian\openlist-windows-amd64\openlist.exe"
$env:OPENLIST_FORCE_BIN_DIR="1"
.\scripts\check_openlist_upload_resume.cmd
```

脚本会把一个小文件拆成两段，用 `Content-Range` 上传并读取直链校验内容。如果探测通过，说明当前 OpenList/驱动组合支持字节级续传；如果不通过，Explorer 会降级为上传暂停、取消、失败后重新上传，不把它标称为字节级断点续传。发布检查默认把不支持视为 WARN；设置 `OPENLIST_REQUIRE_UPLOAD_RESUME=1` 后会把不支持视为 FAIL。

## 多 OpenList 隔离检查

```powershell
.\scripts\check_multi_openlist_state.cmd
```

该脚本会静态检查：

- HTTP 请求是否使用当前 OpenList 实例的地址和 token。
- token 是否按实例写入系统凭据库。
- 云下载任务是否按 `instanceId + remoteId` 隔离。
- 切换 OpenList 后下载页和后台同步是否会重启。

## 云下载说明

Explorer 调用 OpenList 的离线下载接口，不重复实现下载工具协议。OpenList 当前启用了哪些下载工具，以 `/api/public/offline_download_tools` 返回为准。

状态同步会尽量归一化不同工具返回的字段：

- 排队中
- 下载中
- 上传到网盘
- 已暂停
- 已取消
- 失败原因
- 完成目录

如果 SimpleHttp、Aria2、qBittorrent 等工具返回字段不同，任务详情里会同时保留“原始状态”，便于后续兼容。

## 本地数据和凭据

- Token：Windows 桌面版写入 Windows Credential Manager，按 OpenList 实例隔离。
- 设置：SQLite `settings` 表中按 `settings.instances`、`settings.theme`、`settings.aria2RpcPort` 等 key 拆分保存。
- 收藏、历史、任务：使用 SQLite 独立表。
- 预览环境：无法调用 Tauri 命令时回退到 localStorage 或内存 token。

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
