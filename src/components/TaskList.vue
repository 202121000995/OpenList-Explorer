<template>
  <n-data-table
    :columns="columns"
    :data="tasks"
    :bordered="false"
    :single-line="false"
    :pagination="{ pageSize: 12 }"
  />
</template>

<script setup lang="ts">
import { computed, h } from 'vue'
import { NButton, NProgress, NSpace, NTag, type DataTableColumns } from 'naive-ui'
import { FolderOpen, Pause, Play, Trash2, X } from '@lucide/vue'
import { taskStatusLabel, type TransferTask } from '@/models/task'
import { formatBytes } from '@/utils/format'

const props = defineProps<{
  tasks: TransferTask[]
}>()

const emit = defineEmits<{
  pause: [id: string]
  resume: [id: string]
  cancel: [id: string]
  remove: [id: string]
  reveal: [path: string]
}>()

const columns = computed<DataTableColumns<TransferTask>>(() => [
  { title: '名称', key: 'name', minWidth: 220 },
  { title: '路径', key: 'path', minWidth: 260 },
  {
    title: '状态',
    key: 'status',
    width: 110,
    render(row) {
      const type = row.status === 'success' ? 'success' : row.status === 'failed' ? 'error' : 'default'
      return h(NTag, { type, size: 'small' }, { default: () => taskStatusLabel[row.status] })
    }
  },
  {
    title: '进度',
    key: 'progress',
    width: 180,
    render(row) {
      return h(NProgress, { percentage: row.progress, height: 8, processing: row.status === 'running' })
    }
  },
  {
    title: '速度',
    key: 'speed',
    width: 120,
    render(row) {
      return row.speed ? `${formatBytes(row.speed)}/s` : '-'
    }
  },
  {
    title: '',
    key: 'actions',
    width: 212,
    render(row) {
      const localControllable = row.source !== 'openlist-offline'
      return h(NSpace, { justify: 'end', size: 6 }, () => [
        h(
          NButton,
          { circle: true, size: 'small', secondary: true, disabled: !row.localPath, onClick: () => row.localPath && emit('reveal', row.localPath) },
          { icon: () => h(FolderOpen, { size: 15 }) }
        ),
        h(
          NButton,
          { circle: true, size: 'small', secondary: true, disabled: !localControllable, onClick: () => emit('pause', row.id) },
          { icon: () => h(Pause, { size: 15 }) }
        ),
        h(
          NButton,
          { circle: true, size: 'small', secondary: true, disabled: !localControllable, onClick: () => emit('resume', row.id) },
          { icon: () => h(Play, { size: 15 }) }
        ),
        h(
          NButton,
          { circle: true, size: 'small', secondary: true, type: 'error', disabled: !localControllable, onClick: () => emit('cancel', row.id) },
          { icon: () => h(X, { size: 15 }) }
        ),
        h(
          NButton,
          { circle: true, size: 'small', secondary: true, onClick: () => emit('remove', row.id) },
          { icon: () => h(Trash2, { size: 15 }) }
        )
      ])
    }
  }
])
</script>
