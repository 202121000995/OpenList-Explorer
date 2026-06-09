import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import type { TransferStatus, TransferTask, TransferType } from '@/models/task'

export const useTasksStore = defineStore('tasks', () => {
  const tasks = ref<TransferTask[]>([])

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
      createdAt: Date.now()
    }
    tasks.value.unshift(task)
    return task
  }

  function updateTask(id: string, patch: Partial<Pick<TransferTask, 'status' | 'progress' | 'speed' | 'localPath'>>) {
    const task = tasks.value.find((item) => item.id === id)
    if (!task) return
    Object.assign(task, patch)
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

  return {
    tasks,
    uploadTasks,
    downloadTasks,
    addTask,
    updateTask,
    taskById,
    setStatus,
    removeTask,
    clearTasks
  }
})
