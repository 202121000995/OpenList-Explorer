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

function taskMessage(task: OpenListTaskItem, done: boolean) {
  const error = String(task.error ?? task.err ?? '').trim()
  if (error) return error

  const raw = String(task.status ?? task.state ?? '').trim()
  const lower = raw.toLowerCase()
  const target = taskPath(task)
  const prefix = done ? '已完成' : '云下载'

  if (/wait|queue|pending|等待/.test(lower)) return `${prefix}：排队中${target ? `，保存到 ${target}` : ''}`
  if (/upload|上传/.test(lower)) return `${prefix}：正在上传到 OpenList${target ? `，保存到 ${target}` : ''}`
  if (/download|running|active|下载/.test(lower)) return `${prefix}：正在下载${target ? `，保存到 ${target}` : ''}`
  if (/pause|暂停/.test(lower)) return `${prefix}：已暂停${target ? `，保存到 ${target}` : ''}`
  if (/cancel|取消/.test(lower)) return `${prefix}：已取消${target ? `，目标 ${target}` : ''}`
  if (/fail|error|err|失败/.test(lower)) return `${prefix}：失败${target ? `，目标 ${target}` : ''}`
  if (done) return `已完成${target ? `，保存到 ${target}` : ''}`
  return raw ? `${prefix}：${raw}${target ? `，保存到 ${target}` : ''}` : `${prefix}：状态同步中${target ? `，保存到 ${target}` : ''}`
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
      message: taskMessage(item, done)
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
