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
import { NButton } from 'naive-ui'
import TaskList from '@/components/TaskList.vue'
import { cancelTransferTask, pauseTransferTask, resumeTransferTask, revealInFolder } from '@/services/localFile'
import { useTasksStore } from '@/stores/tasks'

const props = defineProps<{
  type: 'upload' | 'download'
}>()

const tasksStore = useTasksStore()
const visibleTasks = computed(() => (props.type === 'upload' ? tasksStore.uploadTasks : tasksStore.downloadTasks))

function openTaskFolder(path: string) {
  revealInFolder(path)
}

async function pauseTask(id: string) {
  tasksStore.setStatus(id, 'paused')
  await pauseTransferTask(id)
}

async function resumeTask(id: string) {
  tasksStore.setStatus(id, 'running')
  await resumeTransferTask(id)
}

async function cancelTask(id: string) {
  tasksStore.setStatus(id, 'canceled')
  await cancelTransferTask(id)
}
</script>
