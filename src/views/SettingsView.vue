<template>
  <div class="settings-view">
    <section class="settings-section">
      <div class="panel-heading">界面设置</div>
      <n-form label-placement="left" label-width="120">
        <n-form-item label="主题">
          <n-radio-group v-model:value="settingsStore.theme">
            <n-radio-button value="light">Light</n-radio-button>
            <n-radio-button value="dark">Dark</n-radio-button>
            <n-radio-button value="auto">Auto</n-radio-button>
          </n-radio-group>
        </n-form-item>
        <n-form-item label="语言">
          <n-select v-model:value="settingsStore.language" :options="languageOptions" />
        </n-form-item>
        <n-form-item label="缓存">
          <n-input :value="settingsStore.cacheSize" readonly />
        </n-form-item>
        <n-form-item label="下载目录">
          <n-input v-model:value="settingsStore.downloadDir" placeholder="留空则使用系统下载目录" />
        </n-form-item>
        <n-form-item label="公网地址">
          <n-input v-model:value="settingsStore.publicBaseUrl" placeholder="例如 https://pan.example.com" />
        </n-form-item>
        <div class="settings-note">获取直链时会用这个地址替换 OpenList 返回的本地地址。</div>
        <n-form-item label="上传线程">
          <n-input-number v-model:value="settingsStore.uploadThreads" :min="1" :max="16" />
        </n-form-item>
        <n-form-item label="下载线程">
          <n-input-number v-model:value="settingsStore.downloadThreads" :min="1" :max="16" />
        </n-form-item>
      </n-form>
    </section>
  </div>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import { useSettingsStore } from '@/stores/settings'
import { defaultDownloadPath } from '@/services/localFile'

const settingsStore = useSettingsStore()

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
