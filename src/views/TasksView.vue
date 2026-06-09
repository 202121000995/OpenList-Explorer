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
        @pause="tasksStore.setStatus($event, 'paused')"
        @resume="tasksStore.setStatus($event, 'running')"
        @cancel="tasksStore.setStatus($event, 'canceled')"
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
import { revealInFolder } from '@/services/localFile'
import { useTasksStore } from '@/stores/tasks'

const props = defineProps<{
  type: 'upload' | 'download'
}>()

const tasksStore = useTasksStore()
const visibleTasks = computed(() => (props.type === 'upload' ? tasksStore.uploadTasks : tasksStore.downloadTasks))

function openTaskFolder(path: string) {
  revealInFolder(path)
}
</script>
