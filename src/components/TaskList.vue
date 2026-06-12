<template>
  <div class="task-card-list">
    <div v-if="!tasks.length" class="task-empty">
      <div class="task-empty-title">暂无任务</div>
      <div class="task-empty-text">上传、下载或云下载任务会显示在这里。</div>
    </div>

    <article v-for="task in tasks" :key="task.id" class="task-card">
      <div class="task-card-icon" :class="task.type">
        <FileUp v-if="task.type === 'upload'" :size="22" />
        <FileDown v-else :size="22" />
      </div>

      <div class="task-card-main">
        <div class="task-card-title-row">
          <button class="task-name-button" type="button" :title="task.name" @click="emit('detail', task.id)">
            {{ task.name }}
          </button>
        </div>

        <div class="task-progress-row">
          <n-tooltip>
            <template #trigger>
              <span class="task-progress-text">{{ progressText(task) }}</span>
            </template>
            {{ progressTooltip(task) }}
          </n-tooltip>
          <n-progress
            class="task-progress"
            :percentage="task.progress"
            :height="6"
            :border-radius="3"
            :show-indicator="false"
            :processing="task.status === 'running'"
          />
        </div>
      </div>

      <div class="task-card-state">
        <span class="task-speed">{{ task.speed ? `${formatBytes(task.speed)}/s` : '-' }}</span>
        <span class="task-status-pill" :class="task.status">{{ taskStatusLabel[task.status] }}</span>
        <button class="task-detail-line" type="button" :title="detailText(task)" @click="emit('detail', task.id)">
          {{ stageText(task) }}
        </button>
      </div>

      <div class="task-card-actions">
        <n-tooltip>
          <template #trigger>
            <n-button
              circle
              size="small"
              secondary
              :disabled="!task.localPath"
              @click="task.localPath && emit('reveal', task.localPath)"
            >
              <template #icon><FolderOpen :size="15" /></template>
            </n-button>
          </template>
          打开所在文件夹
        </n-tooltip>

        <n-tooltip>
          <template #trigger>
            <n-button circle size="small" secondary :disabled="!canPause(task)" @click="emit('pause', task.id)">
              <template #icon><Pause :size="15" /></template>
            </n-button>
          </template>
          暂停
        </n-tooltip>

        <n-tooltip>
          <template #trigger>
            <n-button circle size="small" secondary :disabled="!canResume(task)" @click="emit('resume', task.id)">
              <template #icon><Play :size="15" /></template>
            </n-button>
          </template>
          {{ resumeTitle(task) }}
        </n-tooltip>

        <n-tooltip>
          <template #trigger>
            <n-button
              circle
              size="small"
              secondary
              type="error"
              :disabled="!canCancel(task)"
              @click="emit('cancel', task.id)"
            >
              <template #icon><X :size="15" /></template>
            </n-button>
          </template>
          取消
        </n-tooltip>

        <n-tooltip>
          <template #trigger>
            <n-button circle size="small" secondary @click="emit('remove', task.id)">
              <template #icon><Trash2 :size="15" /></template>
            </n-button>
          </template>
          删除记录
        </n-tooltip>
      </div>
    </article>
  </div>
</template>

<script setup lang="ts">
import { NButton, NProgress, NTooltip } from 'naive-ui'
import { FileDown, FileUp, FolderOpen, Pause, Play, Trash2, X } from '@lucide/vue'
import { taskStageLabel, taskStatusLabel, type TransferTask } from '@/models/task'
import { formatBytes } from '@/utils/format'

defineProps<{
  tasks: TransferTask[]
}>()

const emit = defineEmits<{
  pause: [id: string]
  resume: [id: string]
  cancel: [id: string]
  remove: [id: string]
  reveal: [path: string]
  detail: [id: string]
}>()

function stageText(task: TransferTask) {
  if (task.stage) return taskStageLabel[task.stage]
  return task.source === 'openlist-offline' ? 'OpenList 云下载' : '本地传输'
}

function detailText(task: TransferTask) {
  return [stageText(task), task.message || task.path].filter(Boolean).join(' · ')
}

function progressText(task: TransferTask) {
  if (task.status === 'success') return '已完成'
  if (task.status === 'failed') return task.failureReason || '失败'
  if (task.status === 'canceled') return '已取消'
  return `${task.progress}%`
}

function progressTooltip(task: TransferTask) {
  return [progressText(task), task.failureReason, task.message, task.path].filter(Boolean).join(' 路 ')
}

function canPause(task: TransferTask) {
  return task.source !== 'openlist-offline' && task.status === 'running'
}

function canResume(task: TransferTask) {
  if (task.source === 'openlist-offline') return ['failed', 'canceled'].includes(task.status)
  return ['paused', 'failed', 'canceled'].includes(task.status)
}

function canCancel(task: TransferTask) {
  return ['waiting', 'running', 'paused'].includes(task.status) && (task.source !== 'openlist-offline' || !!task.remoteId)
}

function resumeTitle(task: TransferTask) {
  if (task.source === 'openlist-offline') return '重试'
  if (task.type === 'upload' && ['failed', 'canceled'].includes(task.status)) return '重新上传'
  return '继续'
}
</script>
