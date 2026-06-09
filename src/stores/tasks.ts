import { computed, watch } from 'vue'
import { defineStore } from 'pinia'
import { useStorage } from '@vueuse/core'
import { dbGetJson, dbGetTasks, dbReplaceTasks } from '@/services/database'
import type { TransferSource, TransferStatus, TransferTask, TransferType } from '@/models/task'

interface UpsertRemoteTaskPayload {
  remoteId: string
  source: TransferSource
  type: TransferType
  name: string
  path: string
  status: TransferStatus
  progress: number
  speed?: number
  message?: string
}

export const useTasksStore = defineStore('tasks', () => {
  const tasks = useStorage<TransferTask[]>('openlist.tasks', [])
  let hydrated = false

  const uploadTasks = computed(() => tasks.value.filter((task) => task.type === 'upload'))
  const downloadTasks = computed(() => tasks.value.filter((task) => task.type === 'download'))

  function addTask(type: TransferType, name: string, path: string) {
    const task: TransferTask = {
      id: crypto.randomUUID(),
      type,
      status: 'waiting',
      progress: 0,
      speed: 0,
      path,
      name,
      createdAt: Date.now(),
      updatedAt: Date.now(),
      source: 'local'
    }
    tasks.value.unshift(task)
    return task
  }

  function updateTask(id: string, patch: Partial<Pick<TransferTask, 'status' | 'progress' | 'speed' | 'localPath'>>) {
    const task = tasks.value.find((item) => item.id === id)
    if (!task) return
    Object.assign(task, patch, { updatedAt: Date.now() })
  }

  function upsertRemoteTask(payload: UpsertRemoteTaskPayload) {
    const task = tasks.value.find((item) => item.source === payload.source && item.remoteId === payload.remoteId)
    if (task) {
      Object.assign(task, {
        name: payload.name || task.name,
        path: payload.path || task.path,
        status: payload.status,
        progress: payload.progress,
        speed: payload.speed ?? task.speed,
        message: payload.message,
        updatedAt: Date.now()
      })
      return task
    }

    const next: TransferTask = {
      id: `${payload.source}:${payload.remoteId}`,
      remoteId: payload.remoteId,
      source: payload.source,
      type: payload.type,
      status: payload.status,
      progress: payload.progress,
      speed: payload.speed ?? 0,
      path: payload.path,
      name: payload.name,
      message: payload.message,
      createdAt: Date.now(),
      updatedAt: Date.now()
    }
    tasks.value.unshift(next)
    return next
  }

  function taskById(id: string) {
    return tasks.value.find((task) => task.id === id)
  }

  function setStatus(id: string, status: TransferStatus) {
    updateTask(id, { status })
  }

  function removeTask(id: string) {
    tasks.value = tasks.value.filter((task) => task.id !== id)
  }

  function clearTasks(type?: TransferType) {
    tasks.value = type ? tasks.value.filter((task) => task.type !== type) : []
  }

  async function hydrateFromDatabase() {
    const saved = (await dbGetTasks()) ?? (await dbGetJson<TransferTask[]>('tasks'))
    if (saved) tasks.value = saved
    hydrated = true
  }

  watch(tasks, (value) => {
    if (hydrated) dbReplaceTasks(value)
  }, { deep: true })

  return {
    tasks,
    uploadTasks,
    downloadTasks,
    addTask,
    updateTask,
    taskById,
    setStatus,
    removeTask,
    clearTasks,
    upsertRemoteTask,
    hydrateFromDatabase
  }
})
