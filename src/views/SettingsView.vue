<template>
  <div class="settings-view" :class="{ 'embedded-settings-view': embedded }">
    <section class="settings-section settings-modal-section">
      <div v-if="!embedded" class="panel-heading">界面设置</div>
      <div class="settings-modal-content">
        <div class="settings-group">
          <div class="settings-group-title">界面</div>
          <n-form label-placement="top" size="small">
            <n-form-item label="主题">
              <n-radio-group v-model:value="settingsStore.theme" class="setting-segment">
                <n-radio-button value="light">Light</n-radio-button>
                <n-radio-button value="dark">Dark</n-radio-button>
                <n-radio-button value="auto">Auto</n-radio-button>
              </n-radio-group>
            </n-form-item>
            <n-form-item label="语言">
              <n-select v-model:value="settingsStore.language" :options="languageOptions" />
            </n-form-item>
            <n-form-item label="文件视图">
              <n-radio-group v-model:value="settingsStore.fileViewMode" class="setting-segment">
                <n-radio-button value="rows">列表</n-radio-button>
                <n-radio-button value="grid">网格</n-radio-button>
              </n-radio-group>
            </n-form-item>
          </n-form>
        </div>

        <div class="settings-group">
          <div class="settings-group-title">传输</div>
          <n-form label-placement="top" size="small">
            <n-form-item label="下载目录">
              <n-input v-model:value="settingsStore.downloadDir" placeholder="留空则使用系统下载目录" />
            </n-form-item>
            <div class="settings-inline-fields">
              <label class="settings-compact-field">
                <span>上传线程</span>
                <n-input-number v-model:value="settingsStore.uploadThreads" :min="1" :max="16" />
              </label>
              <label class="settings-compact-field">
                <span>下载线程</span>
                <n-input-number v-model:value="settingsStore.downloadThreads" :min="1" :max="16" />
              </label>
            </div>
          </n-form>
        </div>

        <div class="settings-group">
          <div class="settings-group-title">直链</div>
          <n-form label-placement="top" size="small">
            <n-form-item label="公网地址">
              <n-input v-model:value="settingsStore.publicBaseUrl" placeholder="例如 https://pan.example.com" />
            </n-form-item>
          </n-form>
          <div class="settings-muted-line">只有 OpenList 返回本机或内网地址时，才会用这里的地址替换。</div>
        </div>

        <div class="settings-group">
          <div class="settings-group-title">缓存</div>
          <div class="settings-value-card">
            <span>当前缓存</span>
            <strong>{{ settingsStore.cacheSize }}</strong>
          </div>
          <div class="settings-muted-line">上传支持暂停、取消和失败后重试；字节级断点续传取决于 OpenList 上传接口。</div>
        </div>
      </div>
      <div class="settings-modal-footer">
        <span>设置会自动保存</span>
        <span>{{ settingsStore.effectiveTheme === 'dark' ? '深色模式' : '浅色模式' }}</span>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import { useSettingsStore } from '@/stores/settings'
import { defaultDownloadPath } from '@/services/localFile'

const settingsStore = useSettingsStore()
defineProps<{
  embedded?: boolean
}>()

const languageOptions = [
  { label: '简体中文', value: 'zh-CN' },
  { label: 'English', value: 'en-US' }
]

onMounted(async () => {
  if (!settingsStore.downloadDir) {
    try {
      settingsStore.downloadDir = await defaultDownloadPath()
    } catch {
      // Browser preview cannot read the desktop default path.
    }
  }
})
</script>
