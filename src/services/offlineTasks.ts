import { fsApi, type OpenListTaskItem } from '@/api/fs'
import { useTasksStore } from '@/stores/tasks'
import type { TransferStatus } from '@/models/task'

function taskId(task: OpenListTaskItem, fallback: string) {
  return String(task.id ?? task.tid ?? fallback)
}

function taskName(task: OpenListTaskItem, fallback: string) {
  return String(task.name ?? task.path ?? task.dst ?? fallback)
}

function taskPath(task: OpenListTaskItem) {
  return String(task.path ?? task.dst ?? '')
}

function taskProgress(task: OpenListTaskItem, done: boolean) {
  if (done) return 100
  const value = Number(task.progress ?? task.percentage ?? 0)
  if (!Number.isFinite(value)) return 0
  return Math.max(0, Math.min(99, Math.round(value)))
}

function taskStatus(task: OpenListTaskItem, done: boolean): TransferStatus {
  const raw = String(task.status ?? task.state ?? '').toLowerCase()
  if (done) {
    if (/fail|error|err|失败/.test(raw) || task.error || task.err) return 'failed'
    return 'success'
  }
  if (/wait|queue|pending|等待/.test(raw)) return 'waiting'
  if (/pause|暂停/.test(raw)) return 'paused'
  if (/cancel|取消/.test(raw)) return 'canceled'
  if (/fail|error|err|失败/.test(raw) || task.error || task.err) return 'failed'
  return 'running'
}

function normalizeTasks(value: unknown) {
  if (Array.isArray(value)) return value as OpenListTaskItem[]
  if (value && typeof value === 'object' && Array.isArray((value as { tasks?: unknown[] }).tasks)) {
    return (value as { tasks: OpenListTaskItem[] }).tasks
  }
  return []
}

function syncItems(items: OpenListTaskItem[], done: boolean) {
  const tasksStore = useTasksStore()
  items.forEach((item, index) => {
    const remoteId = taskId(item, `${done ? 'done' : 'undone'}-${index}`)
    tasksStore.upsertRemoteTask({
      remoteId,
      source: 'openlist-offline',
      type: 'download',
      name: taskName(item, `云下载 ${remoteId}`),
      path: taskPath(item),
      status: taskStatus(item, done),
      progress: taskProgress(item, done),
      speed: Number(item.speed ?? 0) || 0,
      message: String(item.error ?? item.err ?? '')
    })
  })
}

export async function syncOfflineDownloadTasks() {
  const [undone, done] = await Promise.allSettled([
    fsApi.offlineDownloadUndoneTasks(),
    fsApi.offlineDownloadDoneTasks()
  ])

  if (undone.status === 'fulfilled') syncItems(normalizeTasks(undone.value), false)
  if (done.status === 'fulfilled') syncItems(normalizeTasks(done.value), true)

  return undone.status === 'fulfilled' || done.status === 'fulfilled'
}
