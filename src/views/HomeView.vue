<template>
  <div class="home-view">
    <section class="storage-overview">
      <button
        v-for="storage in storageStore.storages"
        :key="storage.id"
        type="button"
        class="storage-tile"
        @click="openStorage(storage.id)"
      >
        <span class="storage-logo" :style="{ backgroundColor: storage.color }">{{ storage.iconText }}</span>
        <span class="tile-title">{{ storage.name }}</span>
        <span class="tile-path">{{ storage.usedBytes && storage.totalBytes ? `${formatBytes(storage.usedBytes)} / ${formatBytes(storage.totalBytes)}` : storage.mountPath }}</span>
      </button>
    </section>

    <section class="dashboard-columns">
      <div class="panel">
        <div class="panel-heading">收藏</div>
        <n-empty v-if="favoritesStore.items.length === 0" description="暂无收藏" />
        <button
          v-for="item in favoritesStore.items"
          :key="item.id"
          class="list-row"
          type="button"
          @click="openPath(item.storage, item.path)"
        >
          <Star :size="16" />
          <span>{{ item.path }}</span>
        </button>
      </div>

      <div class="panel">
        <div class="panel-heading">历史记录</div>
        <n-empty v-if="historyStore.items.length === 0" description="暂无历史" />
        <button
          v-for="item in historyStore.items.slice(0, 12)"
          :key="item.id"
          class="list-row"
          type="button"
          @click="openHistoricalPath(item.path)"
        >
          <Clock3 :size="16" />
          <span>{{ item.path }}</span>
        </button>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { useRouter } from 'vue-router'
import { Clock3, Star } from '@lucide/vue'
import { useFavoritesStore } from '@/stores/favorites'
import { useFilesStore } from '@/stores/files'
import { useHistoryStore } from '@/stores/history'
import { useStorageStore } from '@/stores/storage'
import { formatBytes } from '@/utils/format'

const router = useRouter()
const storageStore = useStorageStore()
const filesStore = useFilesStore()
const favoritesStore = useFavoritesStore()
const historyStore = useHistoryStore()

async function openStorage(id: string) {
  storageStore.selectStorage(id)
  filesStore.resetToActiveStorage()
  await router.push({ name: 'files' })
}

async function openPath(storageId: string, path: string) {
  storageStore.selectStorage(storageId)
  await router.push({ name: 'files' })
  await filesStore.load(path)
}

async function openHistoricalPath(path: string) {
  await router.push({ name: 'files' })
  await filesStore.load(path.split('?q=')[0])
}
</script>
