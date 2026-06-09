<template>
  <div class="task-view">
    <div class="panel">
      <div class="panel-heading">
        <span>{{ type === 'upload' ? '上传任务' : '下载任务' }}</span>
        <n-button size="small" secondary :disabled="!visibleTasks.length" @click="tasksStore.clearTasks(type)">
          清空日志
        </n-button>
      </div>
      <TaskList
        :tasks="visibleTasks"
        @pause="pauseTask"
        @resume="resumeTask"
        @cancel="cancelTask"
        @remove="tasksStore.removeTask"
        @reveal="openTaskFolder"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NButton, useMessage } from 'naive-ui'
import TaskList from '@/components/TaskList.vue'
import { fsApi } from '@/api/fs'
import {
  cancelTransferTask,
  downloadWithEngine,
  pauseTransferTask,
  resumeTransferTask,
  revealInFolder,
  uploadWithEngine
} from '@/services/localFile'
import { tokenVault } from '@/services/tokenVault'
import { useHistoryStore } from '@/stores/history'
import { useSettingsStore } from '@/stores/settings'
import { useTasksStore } from '@/stores/tasks'

const props = defineProps<{
  type: 'upload' | 'download'
}>()

const tasksStore = useTasksStore()
const settingsStore = useSettingsStore()
const historyStore = useHistoryStore()
const message = useMessage()
const visibleTasks = computed(() => (props.type === 'upload' ? tasksStore.uploadTasks : tasksStore.downloadTasks))

function openTaskFolder(path: string) {
  revealInFolder(path)
}

async function pauseTask(id: string) {
  tasksStore.setStatus(id, 'paused')
  await pauseTransferTask(id)
}

async function resumeTask(id: string) {
  const task = tasksStore.taskById(id)
  if (!task) return
  const originalStatus = task.status

  if (originalStatus === 'paused' || originalStatus === 'running') {
    tasksStore.setStatus(id, 'running')
    await resumeTransferTask(id)
    return
  }

  if (task.source === 'openlist-offline') return
  if (task.type === 'upload') {
    await resumeUploadTask(task.id)
    return
  }
  await resumeDownloadTask(task.id)
}

async function cancelTask(id: string) {
  tasksStore.setStatus(id, 'canceled')
  await cancelTransferTask(id)
}

async function resumeUploadTask(id: string) {
  const task = tasksStore.taskById(id)
  if (!task?.localPath) {
    message.warning('没有本地文件路径，无法继续上传')
    return
  }
  const token = await tokenVault.getToken(settingsStore.activeInstanceId)
  if (!token) {
    message.warning('请先连接 OpenList')
    return
  }

  try {
    tasksStore.updateTask(id, { status: 'running', progress: Math.max(task.progress, 1) })
    await uploadWithEngine(id, settingsStore.serverUrl, token, task.localPath, task.path)
    tasksStore.updateTask(id, { status: 'success', progress: 100 })
    historyStore.add('upload', task.path)
    message.success(`${task.name} 上传完成`)
  } catch (error) {
    if (tasksStore.taskById(id)?.status === 'canceled') return
    tasksStore.updateTask(id, { status: 'failed', message: error instanceof Error ? error.message : '上传失败' })
    message.error(error instanceof Error ? error.message : '上传失败')
  }
}

async function resumeDownloadTask(id: string) {
  const task = tasksStore.taskById(id)
  if (!task) return

  try {
    tasksStore.updateTask(id, { status: 'running', progress: Math.max(task.progress, 1) })
    let rawUrl = task.remoteUrl
    if (!rawUrl) {
      const detail = await fsApi.get(task.path)
      rawUrl = detail.raw_url || ''
      tasksStore.updateTask(id, { remoteUrl: rawUrl })
    }
    if (!rawUrl) throw new Error('没有可用下载地址')
    const result = await downloadWithEngine(id, rawUrl, task.name, settingsStore.downloadDir)
    tasksStore.updateTask(id, { status: 'success', progress: 100, localPath: result.path })
    historyStore.add('download', task.path)
    message.success(`${task.name} 下载完成`)
  } catch (error) {
    if (tasksStore.taskById(id)?.status === 'canceled') return
    tasksStore.updateTask(id, { status: 'failed', message: error instanceof Error ? error.message : '下载失败' })
    message.error(error instanceof Error ? error.message : '下载失败')
  }
}
</script>
