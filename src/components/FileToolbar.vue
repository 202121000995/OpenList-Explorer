<template>
  <div class="file-toolbar">
    <n-space :wrap="false" align="center">
      <n-tooltip>
        <template #trigger>
          <n-button circle secondary @click="$emit('up')">
            <template #icon><ArrowUp :size="17" /></template>
          </n-button>
        </template>
        返回上级
      </n-tooltip>
      <n-tooltip>
        <template #trigger>
          <n-button circle secondary @click="$emit('refresh')">
            <template #icon><RefreshCw :size="17" /></template>
          </n-button>
        </template>
        刷新
      </n-tooltip>
      <n-tooltip>
        <template #trigger>
          <n-button circle secondary @click="$emit('mkdir')">
            <template #icon><FolderPlus :size="17" /></template>
          </n-button>
        </template>
        新建目录
      </n-tooltip>
      <n-tooltip>
        <template #trigger>
          <n-button circle secondary @click="$emit('upload')">
            <template #icon><Upload :size="17" /></template>
          </n-button>
        </template>
        上传文件
      </n-tooltip>
      <n-tooltip>
        <template #trigger>
          <n-button circle secondary type="error" :disabled="!hasSelection" @click="$emit('delete')">
            <template #icon><Trash2 :size="17" /></template>
          </n-button>
        </template>
        删除
      </n-tooltip>
    </n-space>

    <n-input-group class="search-box">
      <n-input
        :value="keyword"
        clearable
        placeholder="搜索当前目录"
        @update:value="$emit('update:keyword', $event)"
        @keydown.enter="$emit('search')"
      />
      <n-button type="primary" @click="$emit('search')">
        <template #icon><Search :size="16" /></template>
      </n-button>
    </n-input-group>
  </div>
</template>

<script setup lang="ts">
import { ArrowUp, FolderPlus, RefreshCw, Search, Trash2, Upload } from '@lucide/vue'

defineProps<{
  keyword: string
  hasSelection: boolean
}>()

defineEmits<{
  up: []
  refresh: []
  mkdir: []
  upload: []
  delete: []
  search: []
  'update:keyword': [value: string]
}>()
</script>
