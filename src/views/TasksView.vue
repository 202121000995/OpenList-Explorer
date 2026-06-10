<template>
  <div class="task-view" :class="{ 'embedded-task-view': embedded }">
    <div class="panel">
      <div class="panel-heading">
        <span v-if="!embedded">{{ type === 'upload' ? '上传任务' : '下载任务' }}</span>
        <n-space size="small">
          <n-button
            v-if="type === 'download'"
            size="small"
            secondary
            :disabled="!settingsStore.hasToken"
            :loading="syncingCloudTasks"
            @click="() => refreshCloudTasks()"
          >
            刷新云下载状态
          </n-button>
          <n-button size="small" secondary :disabled="!visibleTasks.length" @click="tasksStore.clearTasks(type)">
            清空日志
          </n-button>
        </n-space>
      </div>
      <TaskList
        :tasks="visibleTasks"
        @pause="pauseTask"
        @resume="resumeTask"
        @cancel="cancelTask"
        @remove="removeTask"
        @reveal="openTaskFolder"
        @detail="openTaskDetail"
      />
    </div>

    <n-modal v-model:show="detailDialog">
      <n-card class="modal-card task-detail-card" title="任务详情" role="dialog" aria-modal="true">
        <n-descriptions v-if="detailTask" :column="1" size="small" bordered>
          <n-descriptions-item label="名称">{{ detailTask.name }}</n-descriptions-item>
          <n-descriptions-item label="类型">{{ detailTask.type === 'upload' ? '上传' : '下载' }}</n-descriptions-item>
          <n-descriptions-item label="来源">{{ detailTask.source === 'openlist-offline' ? 'OpenList 云下载' : '本地传输' }}</n-descriptions-item>
          <n-descriptions-item label="状态">{{ detailTask.status }}</n-descriptions-item>
          <n-descriptions-item v-if="detailTask.stage" label="阶段">{{ taskStageLabel[detailTask.stage] }}</n-descriptions-item>
          <n-descriptions-item v-if="detailTask.rawStatus" label="原始状态">{{ detailTask.rawStatus }}</n-descriptions-item>
          <n-descriptions-item label="进度">{{ detailTask.progress }}%</n-descriptions-item>
          <n-descriptions-item label="路径">{{ detailTask.path }}</n-descriptions-item>
          <n-descriptions-item v-if="detailTask.completedDir" label="完成目录">{{ detailTask.completedDir }}</n-descriptions-item>
          <n-descriptions-item v-if="detailTask.failureReason" label="失败原因">{{ detailTask.failureReason }}</n-descriptions-item>
          <n-descriptions-item v-if="detailTask.localPath" label="本地路径">{{ detailTask.localPath }}</n-descriptions-item>
          <n-descriptions-item v-if="detailTask.instanceId" label="OpenList 实例">{{ detailTask.instanceId }}</n-descriptions-item>
          <n-descriptions-item v-if="detailTask.remoteId" label="远程任务 ID">{{ detailTask.remoteId }}</n-descriptions-item>
          <n-descriptions-item label="详情">{{ detailTask.message || '-' }}</n-descriptions-item>
        </n-descriptions>
        <template #footer>
          <n-space justify="end">
            <n-button @click="detailDialog = false">关闭</n-button>
          </n-space>
        </template>
      </n-card>
    </n-modal>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { NButton, NSpace, useMessage } from 'naive-ui'
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
import { syncOfflineDownloadTasks } from '@/services/offlineTasks'
import { useHistoryStore } from '@/stores/history'
import { useSettingsStore } from '@/stores/settings'
import { useTasksStore } from '@/stores/tasks'
import { taskStageLabel } from '@/models/task'

const props = defineProps<{
  type: 'upload' | 'download'
  embedded?: boolean
}>()

const tasksStore = useTasksStore()
const settingsStore = useSettingsStore()
const historyStore = useHistoryStore()
const message = useMessage()
const syncingCloudTasks = ref(false)
const detailDialog = ref(false)
const detailTaskId = ref('')
let syncTimer: number | null = null
const visibleTasks = computed(() => (props.type === 'upload' ? tasksStore.uploadTasks : tasksStore.downloadTasks))
const detailTask = computed(() => tasksStore.taskById(detailTaskId.value))

async function refreshCloudTasks(showMessage = true) {
  if (props.type !== 'download' || syncingCloudTasks.value) return
  syncingCloudTasks.value = true
  try {
    const ok = await syncOfflineDownloadTasks()
    if (showMessage) message[ok ? 'success' : 'warning'](ok ? '云下载状态已刷新' : '未读取到云下载任务状态')
  } catch (error) {
    if (showMessage) message.error(error instanceof Error ? error.message : '云下载状态刷新失败')
  } finally {
    syncingCloudTasks.value = false
  }
}

function restartCloudTaskTimer() {
  if (syncTimer !== null) {
    window.clearInterval(syncTimer)
    syncTimer = null
  }
  if (props.type !== 'download' || !settingsStore.hasToken) return
  refreshCloudTasks(false)
  syncTimer = window.setInterval(() => refreshCloudTasks(false), 5000)
}

function openTaskFolder(path: string) {
  revealInFolder(path)
}

function openTaskDetail(id: string) {
  detailTaskId.value = id
  detailDialog.value = true
}

async function pauseTask(id: string) {
  const task = tasksStore.taskById(id)
  if (task?.source === 'openlist-offline') return
  tasksStore.setStatus(id, 'paused')
  await pauseTransferTask(id)
}

async function resumeTask(id: string) {
  const task = tasksStore.taskById(id)
  if (!task) return
  if (task.source === 'openlist-offline') {
    await retryCloudTask(task.id)
    return
  }
  const originalStatus = task.status

  if (originalStatus === 'paused' || originalStatus === 'running') {
    tasksStore.setStatus(id, 'running')
    await resumeTransferTask(id)
    return
  }

  if (task.type === 'upload') {
    await resumeUploadTask(task.id)
    return
  }
  await resumeDownloadTask(task.id)
}

async function cancelTask(id: string) {
  const task = tasksStore.taskById(id)
  if (task?.source === 'openlist-offline') {
    await cancelCloudTask(task.id)
    return
  }
  tasksStore.setStatus(id, 'canceled')
  await cancelTransferTask(id)
}

async function removeTask(id: string) {
  const task = tasksStore.taskById(id)
  if (task?.source === 'openlist-offline' && task.remoteId) {
    try {
      await fsApi.offlineDownloadTaskAction('delete', task.remoteId)
      await syncOfflineDownloadTasks()
    } catch (error) {
      message.error(error instanceof Error ? error.message : '云下载任务删除失败')
      return
    }
  }
  tasksStore.removeTask(id)
}

async function retryCloudTask(id: string) {
  const task = tasksStore.taskById(id)
  if (!task?.remoteId) return
  try {
    await fsApi.offlineDownloadTaskAction('retry', task.remoteId)
    tasksStore.updateTask(id, { status: 'running', message: '' })
    await syncOfflineDownloadTasks()
    message.success('云下载任务已重试')
  } catch (error) {
    message.error(error instanceof Error ? error.message : '云下载任务重试失败')
  }
}

async function cancelCloudTask(id: string) {
  const task = tasksStore.taskById(id)
  if (!task?.remoteId) return
  try {
    await fsApi.offlineDownloadTaskAction('cancel', task.remoteId)
    tasksStore.updateTask(id, { status: 'canceled' })
    await syncOfflineDownloadTasks()
    message.success('云下载任务已取消')
  } catch (error) {
    message.error(error instanceof Error ? error.message : '云下载任务取消失败')
  }
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
    const text = error instanceof Error ? error.message : '上传失败'
    tasksStore.updateTask(id, {
      status: 'failed',
      message: `${text}。可重试该任务；当前 OpenList 上传接口未确认支持字节级断点续传。`
    })
    message.error(text)
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

onMounted(restartCloudTaskTimer)
watch(() => [props.type, settingsStore.hasToken, settingsStore.activeInstanceId, settingsStore.serverUrl] as const, restartCloudTaskTimer)
onBeforeUnmount(() => {
  if (syncTimer !== null) window.clearInterval(syncTimer)
})
</script>
