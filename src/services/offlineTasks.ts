import { fsApi, type OpenListTaskItem } from '@/api/fs'
import { useSettingsStore } from '@/stores/settings'
import { useTasksStore } from '@/stores/tasks'
import type { TransferStage, TransferStatus } from '@/models/task'

function taskId(task: OpenListTaskItem, fallback: string) {
  return String(task.id ?? task.tid ?? fallback)
}

function taskName(task: OpenListTaskItem, fallback: string) {
  return String(task.name ?? task.path ?? task.dst ?? fallback)
}

function taskPath(task: OpenListTaskItem) {
  return String(task.path ?? task.dst ?? task.dir ?? task.save_path ?? '')
}

function taskRawStatus(task: OpenListTaskItem) {
  return String(task.status ?? task.state ?? task.phase ?? task.stage ?? '').trim()
}

function taskFailureReason(task: OpenListTaskItem) {
  const error = String(task.error ?? task.err ?? '').trim()
  if (error) return error
  const message = String(task.message ?? task.msg ?? '').trim()
  const raw = taskRawStatus(task).toLowerCase()
  return /fail|error|err|失败/.test(raw) ? message : ''
}

function taskCompletedDir(task: OpenListTaskItem) {
  return taskPath(task)
}

function taskStage(task: OpenListTaskItem, done: boolean): TransferStage {
  const raw = taskRawStatus(task).toLowerCase()
  if (done) return taskFailureReason(task) ? 'failed' : 'completed'
  if (/wait|queue|pending|等待/.test(raw)) return 'queued'
  if (/upload|上传/.test(raw)) return 'uploading'
  if (/download|running|active|下载/.test(raw)) return 'downloading'
  if (/pause|暂停/.test(raw)) return 'paused'
  if (/cancel|取消/.test(raw)) return 'canceled'
  if (/fail|error|err|失败/.test(raw) || taskFailureReason(task)) return 'failed'
  return 'syncing'
}

function taskMessage(task: OpenListTaskItem, done: boolean) {
  const failureReason = taskFailureReason(task)
  if (failureReason) return failureReason

  const target = taskPath(task)
  const prefix = done ? '已完成' : '云下载'
  const stage = taskStage(task, done)

  if (stage === 'queued') return `${prefix}：排队中${target ? `，保存到 ${target}` : ''}`
  if (stage === 'uploading') return `${prefix}：正在上传到 OpenList${target ? `，保存到 ${target}` : ''}`
  if (stage === 'downloading') return `${prefix}：正在下载${target ? `，保存到 ${target}` : ''}`
  if (stage === 'paused') return `${prefix}：已暂停${target ? `，保存到 ${target}` : ''}`
  if (stage === 'canceled') return `${prefix}：已取消${target ? `，目标 ${target}` : ''}`
  if (stage === 'failed') return `${prefix}：失败${target ? `，目标 ${target}` : ''}`
  if (done) return `已完成${target ? `，保存到 ${target}` : ''}`
  const raw = taskRawStatus(task)
  return raw ? `${prefix}：${raw}${target ? `，保存到 ${target}` : ''}` : `${prefix}：状态同步中${target ? `，保存到 ${target}` : ''}`
}

function taskProgress(task: OpenListTaskItem, done: boolean) {
  if (done) return 100
  let value = Number(task.progress ?? task.percentage ?? 0)
  if (!Number.isFinite(value)) return 0
  if (value > 0 && value <= 1) value *= 100
  return Math.max(0, Math.min(99, Math.round(value)))
}

function taskStatus(task: OpenListTaskItem, done: boolean): TransferStatus {
  const raw = taskRawStatus(task).toLowerCase()
  if (done) {
    if (/fail|error|err|失败/.test(raw) || taskFailureReason(task)) return 'failed'
    return 'success'
  }
  if (/wait|queue|pending|等待/.test(raw)) return 'waiting'
  if (/pause|暂停/.test(raw)) return 'paused'
  if (/cancel|取消/.test(raw)) return 'canceled'
  if (/fail|error|err|失败/.test(raw) || taskFailureReason(task)) return 'failed'
  return 'running'
}

function normalizeTasks(value: unknown) {
  if (Array.isArray(value)) return value as OpenListTaskItem[]
  if (value && typeof value === 'object' && Array.isArray((value as { tasks?: unknown[] }).tasks)) {
    return (value as { tasks: OpenListTaskItem[] }).tasks
  }
  return []
}

function syncItems(items: OpenListTaskItem[], done: boolean, instanceId: string) {
  const tasksStore = useTasksStore()
  items.forEach((item, index) => {
    const remoteId = taskId(item, `${done ? 'done' : 'undone'}-${index}`)
    tasksStore.upsertRemoteTask({
      remoteId,
      instanceId,
      source: 'openlist-offline',
      type: 'download',
      name: taskName(item, `云下载 ${remoteId}`),
      path: taskPath(item),
      status: taskStatus(item, done),
      stage: taskStage(item, done),
      progress: taskProgress(item, done),
      speed: Number(item.speed ?? 0) || 0,
      rawStatus: taskRawStatus(item),
      failureReason: taskFailureReason(item),
      completedDir: done ? taskCompletedDir(item) : '',
      message: taskMessage(item, done)
    })
  })
}

export async function syncOfflineDownloadTasks() {
  const settingsStore = useSettingsStore()
  settingsStore.ensureInstances()
  const instanceId = settingsStore.activeInstanceId
  const [undone, done] = await Promise.allSettled([
    fsApi.offlineDownloadUndoneTasks(),
    fsApi.offlineDownloadDoneTasks()
  ])

  if (undone.status === 'fulfilled') syncItems(normalizeTasks(undone.value), false, instanceId)
  if (done.status === 'fulfilled') syncItems(normalizeTasks(done.value), true, instanceId)

  return undone.status === 'fulfilled' || done.status === 'fulfilled'
}
