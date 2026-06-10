from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def check(condition: bool, message: str) -> bool:
    print(f"[{'PASS' if condition else 'FAIL'}] {message}")
    return condition


def main() -> int:
    settings = read("src/stores/settings.ts")
    token_vault = read("src/services/tokenVault.ts")
    http = read("src/api/http.ts")
    offline = read("src/services/offlineTasks.ts")
    tasks = read("src/stores/tasks.ts")
    task_model = read("src/models/task.ts")
    tasks_view = read("src/views/TasksView.vue")
    app_layout = read("src/layouts/AppLayout.vue")

    checks = [
        check("activeInstanceId" in settings and "defaultInstanceId" in settings, "settings store keeps active/default OpenList instance ids"),
        check("getToken(settings.activeInstanceId)" in http, "OpenList HTTP uses token for the active instance"),
        check("config.baseURL = settings.serverUrl" in http, "OpenList HTTP uses active instance serverUrl"),
        check("save_openlist_token" in token_vault and "instanceId" in token_vault, "token vault stores credentials per instance"),
        check("instanceId?: string" in task_model, "transfer task model includes instanceId"),
        check("item.instanceId || '') === (payload.instanceId || '')" in tasks, "remote task upsert is isolated by instanceId"),
        check("instanceId," in offline and "settingsStore.activeInstanceId" in offline, "offline task sync records active instanceId"),
        check("settingsStore.activeInstanceId" in tasks_view, "download task view restarts sync when active instance changes"),
        check("settingsStore.activeInstanceId" in app_layout and "restartOfflineTaskSync" in app_layout, "app layout restarts background sync when active instance changes"),
    ]

    return 0 if all(checks) else 1


if __name__ == "__main__":
    raise SystemExit(main())
