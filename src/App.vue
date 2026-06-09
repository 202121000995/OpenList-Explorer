<template>
  <n-config-provider :theme="uiTheme" :locale="zhCN" :date-locale="dateZhCN">
    <n-message-provider>
      <n-dialog-provider>
        <div :class="['app-theme', `theme-${settingsStore.effectiveTheme}`]">
          <RouterView />
        </div>
      </n-dialog-provider>
    </n-message-provider>
  </n-config-provider>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { darkTheme, dateZhCN, zhCN } from 'naive-ui'
import { useSettingsStore } from '@/stores/settings'

const settingsStore = useSettingsStore()

const uiTheme = computed(() => (settingsStore.effectiveTheme === 'dark' ? darkTheme : null))
</script>
