<template>
  <div class="settings-view openlist-settings-view">
    <section class="settings-section">
      <div class="panel-heading">OpenList 实例管理</div>
      <div class="instance-manager">
        <div v-for="instance in settingsStore.instances" :key="instance.id" class="instance-manager-row">
          <n-input
            class="instance-name-input"
            :value="instance.name"
            placeholder="名称"
            @update:value="settingsStore.updateInstance(instance.id, { name: $event })"
          />
          <n-input
            :value="instance.serverUrl"
            placeholder="http://127.0.0.1:5244"
            @update:value="settingsStore.updateInstance(instance.id, { serverUrl: $event })"
          />
          <n-input
            class="instance-user-input"
            :value="instance.username"
            placeholder="用户名"
            @update:value="settingsStore.updateInstance(instance.id, { username: $event })"
          />
          <n-input
            :value="instance.publicBaseUrl"
            placeholder="公网地址"
            @update:value="settingsStore.updateInstance(instance.id, { publicBaseUrl: $event })"
          />
          <span
            class="instance-status"
            :class="instance.lastStatus || 'unknown'"
            :title="instance.lastConnectedAt ? `最后连接：${formatInstanceTime(instance.lastConnectedAt)}` : '尚未成功连接'"
          >
            {{ instanceStatusLabel(instance.lastStatus) }}
          </span>
          <n-button
            secondary
            :type="instance.id === settingsStore.defaultInstanceId ? 'primary' : 'default'"
            @click="setDefaultInstance(instance.id)"
          >
            {{ instance.id === settingsStore.defaultInstanceId ? '默认' : '设默认' }}
          </n-button>
          <n-button
            secondary
            :type="instance.id === settingsStore.activeInstanceId ? 'primary' : 'default'"
            @click="switchInstance(instance.id)"
          >
            {{ instance.id === settingsStore.activeInstanceId ? '当前' : '切换' }}
          </n-button>
          <n-button
            secondary
            type="error"
            :disabled="settingsStore.instances.length <= 1"
            @click="confirmRemoveInstance(instance.id)"
          >
            删除
          </n-button>
        </div>
      </div>
      <n-space justify="end" class="section-actions">
        <n-button @click="addInstance">添加 OpenList</n-button>
      </n-space>
    </section>
    <section class="settings-section">
      <div class="panel-heading">OpenList 连接</div>

      <div class="connection-overview">
        <span class="status-dot" :class="connectionClass" />
        <div>
          <div class="connection-title">{{ connectionTitle }}</div>
          <div class="connection-subtitle">{{ connectionSubtitle }}</div>
        </div>
      </div>

      <n-tabs v-model:value="activeTab" type="segment" animated>
        <n-tab-pane name="builtin" tab="使用内置 OpenList">
          <div class="builtin-panel">
            <n-alert type="info" class="settings-alert">
              适合大多数用户。Explorer 会启动随安装包提供的 OpenList，并自动读取访问凭据。
            </n-alert>
            <n-descriptions :column="1" size="small" bordered>
              <n-descriptions-item label="服务地址">{{ builtinStatus?.server_url ?? 'http://127.0.0.1:5244' }}</n-descriptions-item>
              <n-descriptions-item label="运行状态">{{ builtinStatus?.running ? '已运行' : '未运行' }}</n-descriptions-item>
              <n-descriptions-item label="内置程序">{{ builtinStatus?.available ? '已包含' : '未找到' }}</n-descriptions-item>
              <n-descriptions-item v-if="builtinStatus?.data_dir" label="数据目录">{{ builtinStatus.data_dir }}</n-descriptions-item>
              <n-descriptions-item v-if="builtinAdminPassword" label="Web 管理账号">admin</n-descriptions-item>
              <n-descriptions-item v-if="builtinAdminPassword" label="Web 管理密码">
                <span class="secret-row">
                  <span>{{ builtinAdminPassword }}</span>
                  <n-button size="tiny" secondary @click="copyBuiltinAdminPassword">复制</n-button>
                </span>
              </n-descriptions-item>
            </n-descriptions>
            <n-space justify="end" class="section-actions">
              <n-button :loading="loadingBuiltin" @click="refreshBuiltinStatus()">刷新状态</n-button>
              <n-button v-if="builtinAdminPassword" secondary @click="copyBuiltinAdminPassword">复制管理密码</n-button>
              <n-button type="primary" ghost @click="openBuiltinAdmin">打开管理端</n-button>
              <n-button type="primary" :loading="loadingBuiltin" @click="useBuiltinOpenList">启动并连接</n-button>
            </n-space>
          </div>
        </n-tab-pane>

        <n-tab-pane name="existing" tab="连接已有 OpenList">
          <n-form label-placement="left" label-width="120">
            <n-form-item label="服务器地址">
              <n-input v-model:value="settingsStore.serverUrl" placeholder="http://127.0.0.1:5244" />
            </n-form-item>
            <n-form-item label="账号">
              <n-input v-model:value="usernameInput" placeholder="OpenList 用户名" />
            </n-form-item>
            <n-form-item label="密码">
              <n-input
                v-model:value="passwordInput"
                type="password"
                show-password-on="click"
                placeholder="OpenList 密码"
                @keyup.enter="loginAndTest"
              />
            </n-form-item>
            <n-space justify="end">
              <n-button @click="clearToken">清除连接</n-button>
              <n-button type="primary" :loading="testing" @click="loginAndTest">测试并连接</n-button>
            </n-space>

            <n-collapse class="advanced-token">
              <n-collapse-item title="高级：使用 Token 连接" name="token">
                <n-form-item label="访问令牌">
                  <n-input
                    v-model:value="tokenInput"
                    type="password"
                    show-password-on="click"
                    placeholder="OpenList API Token"
                  />
                </n-form-item>
                <n-alert type="default" class="settings-alert">
                  获取方式：在 OpenList 所在机器执行 openlist.exe admin token --force-bin-dir。
                </n-alert>
                <n-space justify="end">
                  <n-button type="primary" ghost :loading="testing" @click="saveTokenAndTest">保存 Token 并测试</n-button>
                </n-space>
              </n-collapse-item>
            </n-collapse>
          </n-form>
        </n-tab-pane>
      </n-tabs>

      <n-alert v-if="connectionStatus" :type="connectionStatus.type" class="settings-alert">
        {{ connectionStatus.text }}
      </n-alert>
    </section>

    <section class="settings-section">
      <div class="panel-heading">云下载 / Aria2</div>
      <n-alert type="info" class="settings-alert">
        云下载能力由 OpenList 提供。Explorer 会读取 OpenList 已启用的下载工具；如果 OpenList 没有启用 Aria2，这里会明确显示。
      </n-alert>
      <n-form label-placement="left" label-width="120" class="aria2-config-form">
        <n-form-item label="自动启动">
          <n-switch v-model:value="settingsStore.aria2AutoStart" />
        </n-form-item>
        <n-form-item label="RPC 端口">
          <n-input-number v-model:value="settingsStore.aria2RpcPort" :min="1024" :max="65535" />
        </n-form-item>
        <n-form-item label="RPC 密钥">
          <n-input
            v-model:value="settingsStore.aria2RpcSecret"
            type="password"
            show-password-on="click"
            placeholder="留空则不设置 RPC 密钥"
          />
        </n-form-item>
        <n-form-item label="下载目录">
          <n-input v-model:value="settingsStore.aria2DownloadDir" placeholder="留空则使用系统下载目录" />
        </n-form-item>
        <n-form-item label="并发任务">
          <n-input-number v-model:value="settingsStore.aria2MaxConcurrent" :min="1" :max="32" />
        </n-form-item>
        <n-form-item label="连接分片">
          <n-input-number v-model:value="settingsStore.aria2Split" :min="1" :max="32" />
        </n-form-item>
      </n-form>
      <n-descriptions :column="1" size="small" bordered>
        <n-descriptions-item label="OpenList 云下载工具">
          {{ offlineTools.length ? offlineTools.join(', ') : '未检测到' }}
        </n-descriptions-item>
        <n-descriptions-item label="OpenList Aria2">
          {{ openListAria2Text }}
        </n-descriptions-item>
        <n-descriptions-item label="本机 Aria2 RPC">
          {{ aria2Status?.running ? '已运行' : '未运行' }}
        </n-descriptions-item>
        <n-descriptions-item label="随包 Aria2">
          {{ aria2Status?.available ? '已包含' : '未包含' }}
        </n-descriptions-item>
        <n-descriptions-item v-if="aria2Status?.binary_path" label="Aria2 程序">{{ aria2Status.binary_path }}</n-descriptions-item>
        <n-descriptions-item label="RPC 地址">{{ aria2Status?.rpc_url ?? `http://127.0.0.1:${settingsStore.aria2RpcPort}/jsonrpc` }}</n-descriptions-item>
        <n-descriptions-item label="当前下载目录">{{ aria2Status?.download_dir || settingsStore.aria2DownloadDir || '系统下载目录' }}</n-descriptions-item>
      </n-descriptions>
      <n-space justify="end" class="section-actions">
        <n-button :loading="loadingCloudTools" @click="refreshCloudDownloadStatus()">刷新云下载状态</n-button>
        <n-button :loading="startingAria2" @click="startAria2Rpc">启动 Aria2 RPC</n-button>
        <n-button @click="openBuiltinAdmin">打开 OpenList 管理端</n-button>
      </n-space>
    </section>
  </div>
</template>

<script setup lang="ts">
import axios from 'axios'
import { computed, onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useDialog, useMessage } from 'naive-ui'
import { authApi } from '@/api/auth'
import { fsApi } from '@/api/fs'
import { useClipboardAction } from '@/hooks/useClipboard'
import {
  getBuiltinOpenListStatus,
  getLocalAria2Status,
  startLocalAria2,
  startBuiltinOpenList,
  type BuiltinOpenListStatus,
  type LocalAria2Status
} from '@/services/builtinOpenList'
import { useSettingsStore } from '@/stores/settings'
import { useStorageStore } from '@/stores/storage'

type StatusType = 'success' | 'warning' | 'error' | 'info'

const settingsStore = useSettingsStore()
const storageStore = useStorageStore()
const route = useRoute()
const router = useRouter()
const message = useMessage()
const dialog = useDialog()
const { copyText } = useClipboardAction()
const usernameInput = ref(settingsStore.username)
const passwordInput = ref('')
const tokenInput = ref('')
const activeTab = ref(route.query.mode === 'existing' ? 'existing' : 'builtin')
const testing = ref(false)
const loadingBuiltin = ref(false)
const builtinStatus = ref<BuiltinOpenListStatus | null>(null)
const builtinAdminPassword = ref('')
const connectionStatus = ref<{ type: StatusType; text: string } | null>(null)
const aria2Status = ref<LocalAria2Status | null>(null)
const offlineTools = ref<string[]>([])
const loadingCloudTools = ref(false)
const startingAria2 = ref(false)

async function switchInstance(id: string) {
  await settingsStore.switchInstance(id)
  storageStore.clearStorages()
  connectionStatus.value = null
  usernameInput.value = settingsStore.username
  passwordInput.value = ''
  tokenInput.value = ''
  await storageStore.loadFromOpenList()
  settingsStore.markInstanceStatus(id, storageStore.loadError ? 'offline' : 'online')
}

function addInstance() {
  const instance = settingsStore.addInstance({
    name: `OpenList ${settingsStore.instances.length + 1}`,
    serverUrl: 'http://127.0.0.1:5244'
  })
  storageStore.clearStorages()
  usernameInput.value = instance.username
  passwordInput.value = ''
  tokenInput.value = ''
}

function confirmRemoveInstance(id: string) {
  const instance = settingsStore.instances.find((item) => item.id === id)
  if (!instance) return
  const defaultHint = instance.id === settingsStore.defaultInstanceId ? '这是当前默认 OpenList，删除后会自动把第一个实例设为默认。' : ''
  dialog.warning({
    title: '删除 OpenList',
    content: `确认删除“${instance.name}”？保存的访问凭据也会一起删除。${defaultHint}`,
    positiveText: '删除',
    negativeText: '取消',
    onPositiveClick: async () => {
      const removed = await settingsStore.removeInstance(id)
      if (!removed) {
        message.warning('至少需要保留一个 OpenList')
        return
      }
      storageStore.clearStorages()
      usernameInput.value = settingsStore.username
      passwordInput.value = ''
      tokenInput.value = ''
      await storageStore.loadFromOpenList()
      message.success('OpenList 已删除')
    }
  })
}

const connectionClass = computed(() => {
  if (settingsStore.hasToken && storageStore.hasStorages) return 'online'
  if (settingsStore.hasToken) return 'warning'
  return 'offline'
})

const connectionTitle = computed(() => {
  if (settingsStore.hasToken && storageStore.hasStorages) return '已连接 OpenList'
  if (settingsStore.hasToken) return '已保存连接，等待读取存储'
  return '未连接 OpenList'
})

const connectionSubtitle = computed(() => {
  if (storageStore.hasStorages) return `已读取 ${storageStore.storages.length} 个挂载点`
  if (storageStore.loadError) return storageStore.loadError
  return '可以使用内置 OpenList，或连接已有 OpenList。'
})

const openListAria2Text = computed(() => {
  if (offlineTools.value.some((tool) => /aria2/i.test(tool))) return '已启用'
  if (offlineTools.value.length) return '未启用'
  return '未检测到云下载工具'
})

function instanceStatusLabel(status?: string) {
  if (status === 'online') return '在线'
  if (status === 'offline') return '离线'
  return '未知'
}

function formatInstanceTime(value: number) {
  return new Date(value).toLocaleString('zh-CN', { hour12: false })
}

function setDefaultInstance(id: string) {
  if (settingsStore.setDefaultInstance(id)) {
    message.success('默认 OpenList 已更新')
  }
}

function serverBaseUrl() {
  return settingsStore.serverUrl.trim().replace(/\/+$/, '')
}

async function probeOpenListServer() {
  const baseUrl = serverBaseUrl()
  if (!/^https?:\/\//i.test(baseUrl)) {
    throw new Error('服务器地址需要以 http:// 或 https:// 开头')
  }

  try {
    const response = await axios.post(
      `${baseUrl}/api/fs/list`,
      { path: '/', page: 1, per_page: 1, refresh: false },
      { timeout: 5000, validateStatus: () => true }
    )
    const payload = response.data
    const looksLikeOpenList = payload && typeof payload === 'object' && ('code' in payload || 'message' in payload || 'msg' in payload)

    if (!looksLikeOpenList) {
      throw new Error('地址有响应，但没有返回 OpenList API 格式')
    }

    const text = payload.message || payload.msg || `HTTP ${response.status}`
    if (payload.code === 200) return '检测到 OpenList 服务，API 可访问。'
    if (/guest|login|token|登录|未授权|unauthorized/i.test(text)) {
      return '检测到 OpenList 服务，等待登录授权。'
    }
    return `检测到 OpenList 服务：${text}`
  } catch (error) {
    if (error instanceof Error && error.message.includes('OpenList API')) throw error
    throw new Error(`未检测到 OpenList 服务，请检查地址和端口：${baseUrl}`)
  }
}

async function loadStoragesAndGo(messageText: string) {
  await storageStore.loadFromOpenList()
  if (storageStore.loadError) {
    settingsStore.markInstanceStatus(settingsStore.activeInstanceId, 'offline')
    connectionStatus.value = { type: 'warning', text: storageStore.loadError }
    return
  }

  settingsStore.markInstanceStatus(settingsStore.activeInstanceId, 'online')
  connectionStatus.value = { type: 'success', text: messageText }
  message.success(messageText)
  await router.push({ name: 'files' })
}

async function refreshBuiltinStatus(showFeedback = true) {
  loadingBuiltin.value = true
  try {
    builtinStatus.value = await getBuiltinOpenListStatus()
    if (showFeedback) {
      connectionStatus.value = {
        type: builtinStatus.value.available ? 'info' : 'error',
        text: builtinStatus.value.message
      }
      message.info(builtinStatus.value.message)
    }
  } catch {
    builtinStatus.value = {
      available: false,
      running: false,
      server_url: 'http://127.0.0.1:5244',
      message: '当前环境不是 Tauri 桌面端，无法检测内置 OpenList。'
    }
    if (showFeedback) {
      connectionStatus.value = { type: 'error', text: builtinStatus.value.message }
    }
  } finally {
    loadingBuiltin.value = false
  }
}

async function refreshCloudDownloadStatus(showFeedback = true) {
  loadingCloudTools.value = true
  try {
    aria2Status.value = await getLocalAria2Status(settingsStore.aria2RpcPort)
  } catch {
    aria2Status.value = {
      available: false,
      running: false,
      rpc_url: `http://127.0.0.1:${settingsStore.aria2RpcPort}/jsonrpc`,
      rpc_port: settingsStore.aria2RpcPort,
      message: '当前环境无法检测本机 Aria2。'
    }
  }

  try {
    offlineTools.value = await fsApi.offlineDownloadTools()
  } catch {
    offlineTools.value = []
  } finally {
    loadingCloudTools.value = false
  }

  if (showFeedback) {
    message.info(
      offlineTools.value.some((tool) => /aria2/i.test(tool))
        ? 'OpenList 已启用 Aria2 云下载。'
        : 'OpenList 当前未启用 Aria2 云下载。'
    )
  }
}

async function startAria2Rpc() {
  startingAria2.value = true
  try {
    aria2Status.value = await startLocalAria2({
      rpcPort: settingsStore.aria2RpcPort,
      rpcSecret: settingsStore.aria2RpcSecret,
      downloadDir: settingsStore.aria2DownloadDir,
      maxConcurrent: settingsStore.aria2MaxConcurrent,
      split: settingsStore.aria2Split
    })
    message.success(aria2Status.value.message)
  } catch (error) {
    message.error(error instanceof Error ? error.message : 'Aria2 启动失败')
  } finally {
    startingAria2.value = false
  }
}

async function useBuiltinOpenList() {
  loadingBuiltin.value = true
  connectionStatus.value = null
  try {
    await refreshBuiltinStatus(false)
    const session = await startBuiltinOpenList()
    builtinAdminPassword.value = session.admin_password
    settingsStore.serverUrl = session.server_url
    settingsStore.updateInstance(settingsStore.activeInstanceId, {
      name: '本机 OpenList',
      serverUrl: session.server_url,
      username: 'admin',
      isBuiltin: true
    })
    await settingsStore.updateToken(session.token)
    await loadStoragesAndGo('内置 OpenList 已启动并连接成功。')
    await refreshBuiltinStatus(false)
  } catch (error) {
    settingsStore.markInstanceStatus(settingsStore.activeInstanceId, 'offline')
    connectionStatus.value = {
      type: 'error',
      text: error instanceof Error ? error.message : String(error)
    }
  } finally {
    loadingBuiltin.value = false
  }
}

function openBuiltinAdmin() {
  const url = builtinStatus.value?.server_url || settingsStore.serverUrl || 'http://127.0.0.1:5244'
  window.open(url, '_blank')
}

async function copyBuiltinAdminPassword() {
  if (!builtinAdminPassword.value) {
    message.warning('请先启动并连接内置 OpenList')
    return
  }
  await copyText(builtinAdminPassword.value, '管理密码已复制')
}

async function loginAndTest() {
  testing.value = true
  connectionStatus.value = null

  try {
    const probeText = await probeOpenListServer()
    connectionStatus.value = { type: 'info', text: probeText }

    const username = usernameInput.value.trim()
    if (!username || !passwordInput.value) {
      throw new Error('请输入 OpenList 账号和密码')
    }
    settingsStore.username = username
    settingsStore.updateInstance(settingsStore.activeInstanceId, {
      name: settingsStore.activeInstance?.isBuiltin ? '本机 OpenList' : username || '远程 OpenList',
      serverUrl: settingsStore.serverUrl,
      username,
      isBuiltin: false
    })

    const session = await authApi.login({
      username,
      password: passwordInput.value
    })
    if (!session.token) {
      throw new Error('OpenList 登录成功但未返回 Token')
    }

    await settingsStore.updateToken(session.token)
    passwordInput.value = ''
    await loadStoragesAndGo(`${probeText} 已登录并连接成功。`)
  } catch (error) {
    settingsStore.markInstanceStatus(settingsStore.activeInstanceId, 'offline')
    connectionStatus.value = {
      type: 'error',
      text: error instanceof Error ? error.message : String(error)
    }
  } finally {
    testing.value = false
  }
}

async function saveTokenAndTest() {
  if (tokenInput.value.trim()) {
    await settingsStore.updateToken(tokenInput.value)
    tokenInput.value = ''
  }
  await testConnection()
}

async function testConnection() {
  testing.value = true
  connectionStatus.value = null

  try {
    const probeText = await probeOpenListServer()
    if (!settingsStore.hasToken) {
      throw new Error('还没有可用凭据。请使用账号密码“测试并连接”，或在高级区域填写 Token。')
    }
    await fsApi.list({ path: '/', page: 1, per_page: 1, refresh: false })
    await loadStoragesAndGo(`${probeText} 当前凭据可用，已连接。`)
  } catch (error) {
    settingsStore.markInstanceStatus(settingsStore.activeInstanceId, 'offline')
    const text = error instanceof Error ? error.message : '连接失败'
    connectionStatus.value = {
      type: text.includes('storage not found') ? 'warning' : 'error',
      text: text.includes('storage not found')
        ? '连接成功，但 OpenList 还没有添加存储。请先在 OpenList Web 管理端添加挂载。'
        : text
    }
  } finally {
    testing.value = false
  }
}

async function clearToken() {
  await settingsStore.clearToken()
  storageStore.clearStorages()
  settingsStore.username = ''
  usernameInput.value = ''
  passwordInput.value = ''
  tokenInput.value = ''
  connectionStatus.value = null
  message.success('连接信息已清除')
}

onMounted(() => {
  refreshBuiltinStatus(false)
  refreshCloudDownloadStatus(false).then(() => {
    if (settingsStore.aria2AutoStart && aria2Status.value?.available && !aria2Status.value.running) {
      startAria2Rpc()
    }
  })
})
</script>
