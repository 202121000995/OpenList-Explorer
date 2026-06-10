<template>
  <div class="explorer-workbench">
    <StorageSidebar />

    <section
      class="explorer-main"
      :class="{ 'drag-over': dragOver }"
      @dragenter.prevent.stop="dragOver = true"
      @dragover.prevent.stop="dragOver = true"
      @dragleave="dragOver = false"
      @drop.prevent.stop="handleDrop"
    >
      <header class="explorer-header">
        <div class="breadcrumb-row">
          <div class="nav-buttons">
            <button class="icon-button" type="button" @click="filesStore.goUp">
              <ChevronLeft :size="18" />
            </button>
            <button class="icon-button" type="button">
              <ChevronRight :size="18" />
            </button>
            <button class="icon-button" type="button" @click="filesStore.refresh">
              <RefreshCw :size="17" />
            </button>
          </div>

          <div class="breadcrumb-path">
            <button class="breadcrumb-segment root" type="button" @click="loadActiveRoot">
              {{ storageStore.activeStorage?.name }}
            </button>
            <template v-for="part in pathParts" :key="part.path">
              <ChevronRight :size="14" />
              <button class="breadcrumb-segment" type="button" @click="filesStore.load(part.path)">
                {{ part.label }}
              </button>
            </template>
          </div>
        </div>

        <div class="command-row">
          <div class="command-group">
            <button class="command-button primary" type="button" @click="pickFiles">
              <Upload :size="16" />
              <span>上传</span>
            </button>
            <button class="command-button" type="button" :disabled="!storageStore.activeStorage" @click="openCloudDownload">
              <CloudDownload :size="16" />
              <span>云端下载</span>
            </button>
            <button class="command-button" type="button" @click="openMkdir">
              <FolderPlus :size="16" />
              <span>新建文件夹</span>
            </button>
            <button class="command-button" type="button" :disabled="!hasSelection" @click="downloadSelected">
              <Download :size="16" />
              <span>下载</span>
            </button>
            <button class="command-button danger" type="button" :disabled="!hasSelection" @click="confirmDelete()">
              <Trash2 :size="16" />
              <span>删除</span>
            </button>
            <n-dropdown :options="moreOptions" @select="handleMoreSelect">
              <button class="command-button compact" type="button">
                <Ellipsis :size="18" />
                <span>更多</span>
              </button>
            </n-dropdown>
          </div>

          <div class="search-tools">
            <n-input
              v-model:value="filesStore.keyword"
              class="explorer-search"
              clearable
              placeholder="搜索当前目录..."
              @keydown.enter="filesStore.search"
            >
              <template #prefix>
                <Search :size="16" />
              </template>
            </n-input>
            <button class="icon-button bordered" type="button" @click="filesStore.search">
              <ListFilter :size="17" />
            </button>
            <n-dropdown :options="viewOptions" @select="handleViewSelect">
              <button class="icon-button bordered" type="button">
                <Ellipsis :size="17" />
              </button>
            </n-dropdown>
          </div>
        </div>
      </header>

      <div v-if="filesStore.lastError" class="connection-banner">
        <CircleAlert :size="16" />
        <span>{{ filesStore.lastError }}</span>
        <button v-if="!storageStore.activeStorage" class="inline-link" type="button" @click="router.push({ name: 'openlist' })">
          设置 OpenList
        </button>
      </div>

      <div v-if="dragOver" class="drop-hint">
        <UploadCloud :size="24" />
        <span>释放后上传到当前目录</span>
      </div>

      <input ref="fileInput" class="hidden-input" multiple type="file" @change="handleUpload" />
      <input ref="directoryInput" class="hidden-input" multiple type="file" webkitdirectory @change="handleDirectoryUpload" />

      <n-data-table
        v-if="settingsStore.fileViewMode === 'rows'"
        v-model:checked-row-keys="filesStore.selectedPaths"
        class="file-table explorer-table"
        :columns="columns"
        :data="filesStore.sortedFiles"
        :loading="filesStore.loading"
        :row-key="getRowKey"
        :bordered="false"
        :single-line="false"
        :row-props="rowProps"
        table-layout="fixed"
        virtual-scroll
        max-height="calc(100vh - 206px)"
      />

      <div v-else class="file-grid-panel">
        <div v-if="filesStore.loading" class="file-grid-state">正在加载...</div>
        <div v-else-if="!filesStore.sortedFiles.length" class="file-grid-state">当前目录为空</div>
        <div v-else class="file-grid">
          <div
            v-for="file in filesStore.sortedFiles"
            :key="file.path"
            class="file-grid-card"
            :class="{ selected: isSelected(file) }"
            @dblclick="openItem(file)"
            @contextmenu.prevent="showFileContext(file, $event)"
          >
            <input
              class="file-grid-check"
              type="checkbox"
              :checked="isSelected(file)"
              :aria-label="`选择 ${file.name}`"
              @click.stop
              @change="toggleRowSelection(file)"
            />
            <component
              :is="fileIcon(file).component"
              :size="34"
              :class="[fileIcon(file).className, file.type === 'folder' ? 'selectable-folder-icon' : '']"
              @click.stop="toggleRowSelection(file)"
            />
            <button
              class="file-grid-name"
              :class="{ 'folder-name-button': file.type === 'folder' }"
              type="button"
              :title="file.name"
              @click.stop="file.type === 'folder' ? openItem(file) : undefined"
            >
              {{ file.name }}
            </button>
            <span class="file-grid-meta">{{ file.type === 'folder' ? '文件夹' : formatBytes(file.size) }}</span>
            <button class="file-grid-more" type="button" @click.stop="openRowActions(file, $event)">
              <Ellipsis :size="15" />
            </button>
          </div>
        </div>
      </div>

      <n-dropdown
        trigger="manual"
        placement="bottom-start"
        :x="contextMenu.x"
        :y="contextMenu.y"
        :show="contextMenu.show"
        :options="contextOptions"
        @clickoutside="contextMenu.show = false"
        @select="handleContextSelect"
      />

      <n-modal v-model:show="mkdirDialog">
        <n-card class="modal-card" title="新建文件夹" role="dialog" aria-modal="true">
          <n-input v-model:value="folderName" placeholder="文件夹名称" @keydown.enter="submitMkdir" />
          <template #footer>
            <n-space justify="end">
              <n-button @click="mkdirDialog = false">取消</n-button>
              <n-button type="primary" @click="submitMkdir">创建</n-button>
            </n-space>
          </template>
        </n-card>
      </n-modal>

      <n-modal v-model:show="renameDialog">
        <n-card class="modal-card" title="重命名" role="dialog" aria-modal="true">
          <n-input v-model:value="renameValue" placeholder="新名称" @keydown.enter="submitRename" />
          <template #footer>
            <n-space justify="end">
              <n-button @click="renameDialog = false">取消</n-button>
              <n-button type="primary" @click="submitRename">保存</n-button>
            </n-space>
          </template>
        </n-card>
      </n-modal>

      <n-modal v-model:show="transferDialog">
        <n-card class="modal-card" :title="transferMode === 'copy' ? '复制到' : '移动到'" role="dialog" aria-modal="true">
          <n-space vertical>
            <n-input v-model:value="destinationPath" placeholder="目标目录路径，例如 /OneDrive/备份" @keydown.enter="submitTransfer" />
            <div class="directory-picker">
              <div class="directory-picker-toolbar">
                <n-button size="small" secondary @click="loadTransferFolders(dirname(destinationPath) || '/')">上一级</n-button>
                <n-button size="small" secondary :loading="transferFoldersLoading" @click="loadTransferFolders(destinationPath)">刷新</n-button>
              </div>
              <div class="directory-tree-shell">
                <n-tree
                  block-line
                  :data="transferTreeData"
                  :selected-keys="[destinationPath]"
                  :on-load="loadTransferTreeNode"
                  @update:selected-keys="handleTransferTreeSelect"
                />
              </div>
              <div v-if="transferFoldersLoading" class="directory-picker-state">正在读取目录...</div>
              <div v-else-if="!transferFolders.length" class="directory-picker-state">当前目录没有子文件夹</div>
              <template v-else>
                <button
                  v-for="folder in transferFolders"
                  :key="folder.path"
                  class="directory-picker-row"
                  type="button"
                  @click="loadTransferFolders(folder.path)"
                >
                  <Folder :size="17" class="folder-icon" />
                  <span>{{ folder.name }}</span>
                </button>
              </template>
            </div>
          </n-space>
          <template #footer>
            <n-space justify="space-between" align="center">
              <span class="modal-help">{{ filesStore.selectedPaths.length }} 个项目</span>
              <n-space>
                <n-button @click="transferDialog = false">取消</n-button>
                <n-button type="primary" @click="submitTransfer">确认</n-button>
              </n-space>
            </n-space>
          </template>
        </n-card>
      </n-modal>

      <n-modal v-model:show="cloudDownloadDialog">
        <n-card class="modal-card" title="云端下载" role="dialog" aria-modal="true">
          <n-space vertical>
            <n-alert :type="cloudAria2Enabled ? 'success' : 'warning'">
              {{ cloudDownloadHint }}
            </n-alert>
            <div v-if="!cloudAria2Enabled" class="cloud-aria2-guide">
              <div>Aria2 启用入口在 OpenList 管理端，不在这个下载弹窗里。</div>
              <div class="cloud-aria2-config">
                <span>RPC 地址：{{ aria2RpcUrl }}</span>
                <span>RPC 密钥：{{ settingsStore.aria2RpcSecret || '未设置' }}</span>
              </div>
              <n-space size="small">
                <n-button size="small" secondary @click="copyAria2ConfigForOpenList">复制 Aria2 配置</n-button>
                <n-button size="small" type="primary" ghost @click="openOpenListAdminForAria2">打开 OpenList 管理端</n-button>
              </n-space>
            </div>
            <n-input v-model:value="cloudUrls" type="textarea" placeholder="每行一个下载地址" :autosize="{ minRows: 4, maxRows: 8 }" />
            <n-select v-model:value="cloudTool" :options="cloudToolOptions" placeholder="下载工具" />
            <n-input v-model:value="cloudTargetPath" placeholder="保存目录" />
          </n-space>
          <template #footer>
            <n-space justify="end">
              <n-button @click="cloudDownloadDialog = false">取消</n-button>
              <n-button type="primary" :loading="cloudSubmitting" @click="submitCloudDownload">提交下载</n-button>
            </n-space>
          </template>
        </n-card>
      </n-modal>

      <n-modal v-model:show="propertiesDialog">
        <n-card class="modal-card file-properties-card" title="属性" role="dialog" aria-modal="true">
          <n-descriptions v-if="propertiesFile" :column="1" size="small" bordered>
            <n-descriptions-item label="名称">{{ propertiesFile.name }}</n-descriptions-item>
            <n-descriptions-item label="类型">{{ fileKindLabel(propertiesFile) }}</n-descriptions-item>
            <n-descriptions-item label="大小">{{ propertiesFile.type === 'folder' ? '-' : formatBytes(propertiesFile.size) }}</n-descriptions-item>
            <n-descriptions-item label="修改时间">{{ formatDate(propertiesFile.modifiedAt) }}</n-descriptions-item>
            <n-descriptions-item label="存储">{{ storageStore.activeStorage?.name || '-' }}</n-descriptions-item>
            <n-descriptions-item label="路径">
              <span class="secret-row">
                <span>{{ propertiesFile.path }}</span>
                <n-button size="tiny" secondary @click="copyText(propertiesFile.path, '路径已复制')">复制</n-button>
              </span>
            </n-descriptions-item>
            <n-descriptions-item v-if="propertiesRawUrl" label="直链">
              <span class="secret-row">
                <span>{{ propertiesRawUrl }}</span>
                <n-button size="tiny" secondary @click="copyText(propertiesRawUrl, '直链已复制')">复制</n-button>
              </span>
            </n-descriptions-item>
            <n-descriptions-item v-if="propertiesProbeText" label="直链校验">
              {{ propertiesProbeText }}
            </n-descriptions-item>
            <n-descriptions-item v-if="propertiesQrDataUrl" label="二维码">
              <img class="raw-url-qr" :src="propertiesQrDataUrl" alt="直链二维码" />
            </n-descriptions-item>
            <n-descriptions-item v-else-if="propertiesLoading" label="直链">正在获取...</n-descriptions-item>
          </n-descriptions>
          <template #footer>
            <n-space justify="space-between">
              <n-space>
                <n-button :disabled="!propertiesRawUrl" @click="openPropertiesRawUrl">打开直链</n-button>
                <n-button :disabled="!propertiesRawUrl" :loading="propertiesProbeLoading" @click="probePropertiesRawUrl">校验直链</n-button>
                <n-button :disabled="!propertiesRawUrl" :loading="propertiesQrLoading" @click="generatePropertiesQr">生成二维码</n-button>
              </n-space>
              <n-button @click="propertiesDialog = false">关闭</n-button>
            </n-space>
          </template>
        </n-card>
      </n-modal>
    </section>
  </div>
</template>

<script setup lang="ts">
import { computed, h, onBeforeUnmount, onMounted, reactive, ref } from 'vue'
import QRCode from 'qrcode'
import { useRouter } from 'vue-router'
import { getCurrentWindow } from '@tauri-apps/api/window'
import type { UnlistenFn } from '@tauri-apps/api/event'
import {
  NButton,
  NIcon,
  NSpace,
  useDialog,
  useMessage,
  type DataTableColumns,
  type DropdownOption,
  type TreeOption
} from 'naive-ui'
import type { Component } from 'vue'
import {
  Archive,
  ChevronLeft,
  ChevronRight,
  CircleAlert,
  CloudDownload,
  Copy,
  Download,
  Ellipsis,
  File,
  FileImage,
  FileText,
  Folder,
  FolderPlus,
  Grid2X2,
  Info,
  Link,
  ListFilter,
  MoveRight,
  Pencil,
  RefreshCw,
  Rows3,
  Search,
  Star,
  Trash2,
  Upload,
  UploadCloud
} from '@lucide/vue'
import StorageSidebar from '@/components/StorageSidebar.vue'
import { fsApi } from '@/api/fs'
import { openExternalUrl } from '@/services/builtinOpenList'
import type { ExplorerFileItem } from '@/models/file'
import { useClipboardAction } from '@/hooks/useClipboard'
import { useFavoritesStore } from '@/stores/favorites'
import { useFilesStore } from '@/stores/files'
import { useHistoryStore } from '@/stores/history'
import { useSettingsStore } from '@/stores/settings'
import { useStorageStore } from '@/stores/storage'
import { useTasksStore } from '@/stores/tasks'
import {
  downloadWithEngine,
  expandUploadPaths,
  probeUrl,
  selectUploadFiles,
  uploadWithEngine,
  type LocalUploadSelection
} from '@/services/localFile'
import { tokenVault } from '@/services/tokenVault'
import { syncOfflineDownloadTasks } from '@/services/offlineTasks'
import type { TransferTask } from '@/models/task'
import { formatBytes, formatDate } from '@/utils/format'
import { dirname, joinPath } from '@/utils/path'

type TransferMode = 'copy' | 'move'
type DownloadQueueItem = {
  file: ExplorerFileItem
  task: TransferTask
  relativePath?: string
}
type UploadQueueItem = {
  selection: LocalUploadSelection
  task: TransferTask
}

const filesStore = useFilesStore()
const router = useRouter()
const favoritesStore = useFavoritesStore()
const settingsStore = useSettingsStore()
const storageStore = useStorageStore()
const tasksStore = useTasksStore()
const historyStore = useHistoryStore()
const dialog = useDialog()
const message = useMessage()
const { copyText } = useClipboardAction()

const fileInput = ref<HTMLInputElement | null>(null)
const directoryInput = ref<HTMLInputElement | null>(null)
const mkdirDialog = ref(false)
const renameDialog = ref(false)
const transferDialog = ref(false)
const cloudDownloadDialog = ref(false)
const dragOver = ref(false)
const transferMode = ref<TransferMode>('copy')
const folderName = ref('')
const renameValue = ref('')
const destinationPath = ref('')
const transferFolders = ref<Array<{ name: string; path: string }>>([])
const transferTreeData = ref<TreeOption[]>([])
const transferFoldersLoading = ref(false)
const cloudUrls = ref('')
const cloudTool = ref('SimpleHttp')
const cloudTools = ref<string[]>(['SimpleHttp'])
const cloudTargetPath = ref('')
const cloudSubmitting = ref(false)
const activeFile = ref<ExplorerFileItem | null>(null)
const propertiesDialog = ref(false)
const propertiesFile = ref<ExplorerFileItem | null>(null)
const propertiesRawUrl = ref('')
const propertiesLoading = ref(false)
const propertiesProbeLoading = ref(false)
const propertiesProbeText = ref('')
const propertiesQrLoading = ref(false)
const propertiesQrDataUrl = ref('')
let unlistenDragDrop: UnlistenFn | null = null

function delay(ms: number) {
  return new Promise((resolve) => window.setTimeout(resolve, ms))
}

async function waitForRunnableTask(taskId: string) {
  while (true) {
    const task = tasksStore.taskById(taskId)
    if (!task || task.status === 'canceled') return false
    if (task.status !== 'paused') return true
    await delay(250)
  }
}

async function runLimited<T>(items: T[], limit: number, runner: (item: T) => Promise<void>) {
  const workerCount = Math.max(1, Math.min(limit || 1, items.length))
  let index = 0
  await Promise.all(
    Array.from({ length: workerCount }, async () => {
      while (index < items.length) {
        const item = items[index]
        index += 1
        await runner(item)
      }
    })
  )
}

function mapListedFile(parentPath: string, item: ExplorerFileItem['raw']): ExplorerFileItem {
  const path = joinPath(parentPath, item.name)
  return {
    id: path,
    name: item.name,
    path,
    type: item.is_dir ? 'folder' : 'file',
    size: item.size,
    modifiedAt: item.modified,
    raw: item
  }
}

async function collectDownloadFiles(folder: ExplorerFileItem, relativeRoot = folder.name): Promise<Array<{ file: ExplorerFileItem; relativePath: string }>> {
  const response = await fsApi.list({ path: folder.path, page: 1, per_page: 1000, refresh: false })
  const result: Array<{ file: ExplorerFileItem; relativePath: string }> = []
  for (const item of response.content ?? []) {
    const child = mapListedFile(folder.path, item)
    const relativePath = joinPath(relativeRoot, child.name)
    if (child.type === 'folder') {
      result.push(...(await collectDownloadFiles(child, relativePath)))
    } else {
      result.push({ file: child, relativePath })
    }
  }
  return result
}

const contextMenu = reactive({
  show: false,
  x: 0,
  y: 0
})

const hasSelection = computed(() => filesStore.selectedPaths.length > 0)
const pathParts = computed(() => {
  const root = storageStore.activeStorage?.mountPath ?? ''
  if (!root) return [{ label: '未连接', path: '' }]
  const relativePath = filesStore.currentPath.startsWith(root)
    ? filesStore.currentPath.slice(root.length)
    : filesStore.currentPath
  const parts = relativePath.split('/').filter(Boolean)
  const result: Array<{ label: string; path: string }> = []

  parts.reduce((path, part) => {
    const nextPath = joinPath(path, part)
    result.push({ label: part, path: nextPath })
    return nextPath
  }, root)

  if (result.length === 0) {
    result.push({ label: '我的文件', path: root })
  }

  return result
})

const moreOptions = computed<DropdownOption[]>(() => [
  { label: '上传目录', key: 'uploadDirectory', icon: renderIcon(Upload), disabled: !storageStore.activeStorage },
  { label: '复制到...', key: 'copy', icon: renderIcon(Copy), disabled: !hasSelection.value },
  { label: '移动到...', key: 'move', icon: renderIcon(MoveRight), disabled: !hasSelection.value },
  { label: '复制路径', key: 'copyPath', icon: renderIcon(Copy), disabled: !hasSelection.value },
  { label: '刷新目录', key: 'refresh', icon: renderIcon(RefreshCw) }
])

const cloudToolOptions = computed<Array<{ label: string; value: string; disabled?: boolean }>>(() => {
  const options: Array<{ label: string; value: string; disabled?: boolean }> = cloudTools.value.map((tool) => ({ label: tool, value: tool }))
  if (!cloudTools.value.some((tool) => /aria2/i.test(tool))) {
    options.push({ label: 'Aria2（OpenList 未启用）', value: 'Aria2', disabled: true })
  }
  return options
})
const cloudAria2Enabled = computed(() => cloudTools.value.some((tool) => /aria2/i.test(tool)))
const aria2RpcUrl = computed(() => `http://127.0.0.1:${settingsStore.aria2RpcPort}/jsonrpc`)
const cloudDownloadHint = computed(() => {
  const tools = cloudTools.value.length ? cloudTools.value.join('、') : '未检测到'
  if (cloudAria2Enabled.value) return `当前 OpenList 已启用 Aria2。可用云下载工具：${tools}。`
  return `当前 OpenList 可用云下载工具：${tools}。未显示 Aria2 表示需要先到 OpenList 管理端启用 Aria2。`
})

const viewOptions = computed<DropdownOption[]>(() => [
  { label: settingsStore.fileViewMode === 'rows' ? '列表排列（当前）' : '列表排列', key: 'rows', icon: renderIcon(Rows3) },
  { label: settingsStore.fileViewMode === 'grid' ? '网格排列（当前）' : '网格排列', key: 'grid', icon: renderIcon(Grid2X2) },
  { type: 'divider', key: 'view-divider' },
  { label: sortOptionLabel('name', 'asc', '名称 A-Z'), key: 'sort:name:asc' },
  { label: sortOptionLabel('name', 'desc', '名称 Z-A'), key: 'sort:name:desc' },
  { label: sortOptionLabel('size', 'asc', '大小 从小到大'), key: 'sort:size:asc' },
  { label: sortOptionLabel('size', 'desc', '大小 从大到小'), key: 'sort:size:desc' },
  { label: sortOptionLabel('modifiedAt', 'desc', '时间 最新优先'), key: 'sort:modifiedAt:desc' },
  { label: sortOptionLabel('modifiedAt', 'asc', '时间 最早优先'), key: 'sort:modifiedAt:asc' }
])

const contextOptions = computed<DropdownOption[]>(() => {
  const file = activeFile.value
  if (!file) return []

  return [
    { label: file.type === 'folder' ? '打开' : '打开直链', key: 'open', icon: renderIcon(Folder) },
    { label: '下载', key: 'download', icon: renderIcon(Download) },
    { label: '复制直链', key: 'copyRawUrl', icon: renderIcon(Link), disabled: file.type === 'folder' },
    { label: '复制路径', key: 'copyPath', icon: renderIcon(Copy) },
    { label: '收藏', key: 'favorite', icon: renderIcon(Star) },
    { label: '重命名', key: 'rename', icon: renderIcon(Pencil) },
    { label: '属性', key: 'properties', icon: renderIcon(Info) },
    { label: '删除', key: 'delete', icon: renderIcon(Trash2) }
  ]
})

const columns: DataTableColumns<ExplorerFileItem> = [
  { type: 'selection', width: 38 },
  {
    title: '名称',
    key: 'name',
    minWidth: 180,
    ellipsis: { tooltip: true },
    render(row) {
      const icon = fileIcon(row)
      return h('div', { class: 'name-cell' }, [
        h(icon.component, {
          size: 18,
          class: [icon.className, row.type === 'folder' ? 'selectable-folder-icon' : ''],
          onClick: (event: MouseEvent) => {
            if (row.type !== 'folder') return
            event.stopPropagation()
            toggleRowSelection(row)
          }
        }),
        h(
          'button',
          {
            class: ['name-button', row.type === 'folder' ? 'folder-name-button' : ''],
            type: 'button',
            onClick: (event: MouseEvent) => {
              if (row.type !== 'folder') return
              event.stopPropagation()
              openItem(row)
            }
          },
          row.name
        ),
        favoritesStore.isFavorite(storageStore.activeStorageId, row.path)
          ? h(Star, { size: 14, class: 'favorite-icon' })
          : null
      ])
    }
  },
  {
    title: '大小',
    key: 'size',
    width: 86,
    render(row) {
      return row.type === 'folder' ? '-' : formatBytes(row.size)
    }
  },
  {
    title: '类型',
    key: 'type',
    width: 96,
    ellipsis: { tooltip: true },
    render(row) {
      return fileKindLabel(row)
    }
  },
  {
    title: '修改时间',
    key: 'modifiedAt',
    width: 156,
    ellipsis: { tooltip: true },
    render(row) {
      return formatDate(row.modifiedAt)
    }
  },
  {
    title: '操作',
    key: 'actions',
    width: 72,
    align: 'center',
    render(row) {
      return h(NSpace, { justify: 'center', size: 8, wrap: false }, () => [
        h(
          NButton,
          { circle: true, secondary: true, size: 'small', onClick: () => openRowActions(row) },
          { icon: () => h(Ellipsis, { size: 15 }) }
        )
      ])
    }
  }
]

function renderIcon(icon: Component) {
  return () => h(NIcon, null, { default: () => h(icon, { size: 16 }) })
}

function sortOptionLabel(key: typeof filesStore.sortKey, order: typeof filesStore.sortOrder, label: string) {
  return filesStore.sortKey === key && filesStore.sortOrder === order ? `${label}（当前）` : label
}

function fileIcon(row: ExplorerFileItem) {
  if (row.type === 'folder') return { component: Folder, className: 'folder-icon' }
  if (/\.(png|jpe?g|gif|webp|svg)$/i.test(row.name)) return { component: FileImage, className: 'image-icon' }
  if (/\.(zip|rar|7z|tar|gz)$/i.test(row.name)) return { component: Archive, className: 'archive-icon' }
  if (/\.(txt|md|json|csv|log)$/i.test(row.name)) return { component: FileText, className: 'text-icon' }
  return { component: File, className: 'file-icon' }
}

function fileKindLabel(row: ExplorerFileItem) {
  if (row.type === 'folder') return '文件夹'
  if (/\.(png|jpe?g|gif|webp|svg)$/i.test(row.name)) return '图片'
  if (/\.(zip|rar|7z|tar|gz)$/i.test(row.name)) return '压缩文件'
  if (/\.(txt|md|json|csv|log)$/i.test(row.name)) return '文本文档'
  return '文件'
}

function rowProps(row: ExplorerFileItem) {
  return {
    onDblclick: () => openItem(row),
    onContextmenu: (event: MouseEvent) => {
      event.preventDefault()
      showFileContext(row, event)
    }
  }
}

function showFileContext(row: ExplorerFileItem, event: MouseEvent) {
  activeFile.value = row
  contextMenu.show = false
  requestAnimationFrame(() => {
    contextMenu.x = event.clientX
    contextMenu.y = event.clientY
    contextMenu.show = true
  })
}

function openRowActions(row: ExplorerFileItem, event?: MouseEvent) {
  activeFile.value = row
  contextMenu.show = false
  const target = event?.currentTarget instanceof HTMLElement ? event.currentTarget : document.activeElement
  const button = target instanceof HTMLElement ? target.getBoundingClientRect() : null
  requestAnimationFrame(() => {
    contextMenu.x = button ? button.left : window.innerWidth - 120
    contextMenu.y = button ? button.bottom + 6 : 120
    contextMenu.show = true
  })
}

function getRowKey(row: ExplorerFileItem) {
  return row.path
}

function isSelected(row: ExplorerFileItem) {
  return filesStore.selectedPaths.includes(row.path)
}

function toggleRowSelection(row: ExplorerFileItem) {
  const selected = new Set(filesStore.selectedPaths)
  if (selected.has(row.path)) selected.delete(row.path)
  else selected.add(row.path)
  filesStore.selectedPaths = Array.from(selected)
}

async function loadActiveRoot() {
  if (!storageStore.activeStorage) {
    await router.push({ name: 'openlist' })
    return
  }
  await filesStore.load(storageStore.activeStorage.mountPath)
}

async function openItem(file: ExplorerFileItem) {
  if (file.type === 'folder') {
    await filesStore.enter(file)
    return
  }

  const rawUrl = await filesStore.getRawUrl(file)
  if (rawUrl) window.open(rawUrl, '_blank')
}

async function downloadFile(file: ExplorerFileItem, existingTask?: TransferTask, relativePath?: string) {
  if (file.type === 'folder') return
  const task = existingTask ?? tasksStore.addTask('download', file.name, file.path)
  const canRun = await waitForRunnableTask(task.id)
  if (!canRun) return
  tasksStore.updateTask(task.id, { status: 'running', progress: 5 })
  message.info(`${file.name} 已加入下载列表`)

  const rawUrl = await filesStore.getRawUrl(file)
  if (!rawUrl) {
    tasksStore.updateTask(task.id, { status: 'failed' })
    message.error('没有获取到下载地址')
    return
  }

  try {
    tasksStore.updateTask(task.id, { progress: 1, remoteUrl: rawUrl })
    const result = await downloadWithEngine(task.id, rawUrl, file.name, settingsStore.downloadDir, relativePath)
    if (tasksStore.taskById(task.id)?.status === 'canceled') return
    tasksStore.updateTask(task.id, { status: 'success', progress: 100, localPath: result.path })
    historyStore.add('download', file.path)
    message.success(`${file.name} 下载完成`)
  } catch (error) {
    tasksStore.updateTask(task.id, { status: 'failed' })
    message.error(error instanceof Error ? error.message : '下载失败')
  }
}

async function downloadSelected() {
  const entries: Array<{ file: ExplorerFileItem; relativePath?: string }> = []
  for (const file of filesStore.selectedFiles) {
    if (file.type === 'folder') {
      entries.push(...(await collectDownloadFiles(file)))
    } else {
      entries.push({ file })
    }
  }

  const downloads: DownloadQueueItem[] = entries.map(({ file, relativePath }) => ({
    file,
    relativePath,
    task: tasksStore.addTask('download', relativePath ?? file.name, file.path)
  }))
  if (!downloads.length) return
  message.info(`${downloads.length} 个文件已加入下载列表`)
  await runLimited(downloads, settingsStore.downloadThreads, ({ file, task, relativePath }) => downloadFile(file, task, relativePath))
}

async function downloadOne(file: ExplorerFileItem) {
  if (file.type === 'folder') {
    const entries = await collectDownloadFiles(file)
    const downloads: DownloadQueueItem[] = entries.map(({ file, relativePath }) => ({
      file,
      relativePath,
      task: tasksStore.addTask('download', relativePath, file.path)
    }))
    if (!downloads.length) {
      message.warning('文件夹为空')
      return
    }
    message.info(`${downloads.length} 个文件已加入下载列表`)
    await runLimited(downloads, settingsStore.downloadThreads, ({ file, task, relativePath }) => downloadFile(file, task, relativePath))
    return
  }

  await downloadFile(file)
}

async function copySelectedPath() {
  const text = filesStore.selectedPaths.join('\n')
  if (text) await copyText(text, '路径已复制')
}

function handleMoreSelect(key: string) {
  if (key === 'uploadDirectory') pickDirectory()
  if (key === 'copy') openTransfer('copy')
  if (key === 'move') openTransfer('move')
  if (key === 'copyPath') copySelectedPath()
  if (key === 'refresh') filesStore.refresh()
}

function handleViewSelect(key: string) {
  if (key === 'rows' || key === 'grid') settingsStore.fileViewMode = key
  if (key.startsWith('sort:')) {
    const [, sortKey, sortOrder] = key.split(':')
    if ((sortKey === 'name' || sortKey === 'size' || sortKey === 'modifiedAt') && (sortOrder === 'asc' || sortOrder === 'desc')) {
      filesStore.sortKey = sortKey
      filesStore.sortOrder = sortOrder
    }
  }
}

async function handleContextSelect(key: string) {
  contextMenu.show = false
  const file = activeFile.value
  if (!file) return

  if (key === 'open') await openItem(file)
  if (key === 'download') await downloadOne(file)
  if (key === 'copyPath') await copyText(file.path, '路径已复制')
  if (key === 'copyRawUrl') await copyRawUrl(file)
  if (key === 'favorite') favoritesStore.toggle(storageStore.activeStorageId, file.path)
  if (key === 'rename') startRename(file)
  if (key === 'properties') openProperties(file)
  if (key === 'delete') confirmDelete(file)
}

async function copyRawUrl(file: ExplorerFileItem) {
  if (file.type === 'folder') {
    await copyText(file.path, '路径已复制')
    return
  }

  const rawUrl = await filesStore.getRawUrl(file)
  if (rawUrl) await copyText(rawUrl, '直链已复制')
}

async function copyAria2ConfigForOpenList() {
  const lines = [
    `RPC 地址：${aria2RpcUrl.value}`,
    `RPC 端口：${settingsStore.aria2RpcPort}`,
    `RPC 密钥：${settingsStore.aria2RpcSecret || '未设置'}`,
    `下载目录：${settingsStore.downloadDir || '系统下载目录'}`
  ]
  await copyText(lines.join('\n'), 'Aria2 配置已复制')
}

async function openOpenListAdminForAria2() {
  try {
    await openExternalUrl(settingsStore.serverUrl)
    message.info('在 OpenList 管理端进入离线下载工具设置，新增或启用 Aria2，并填入刚复制的 RPC 配置。')
  } catch (error) {
    message.error(error instanceof Error ? error.message : '管理端打开失败')
  }
}

function openMkdir() {
  folderName.value = ''
  mkdirDialog.value = true
}

async function openProperties(file: ExplorerFileItem) {
  propertiesFile.value = file
  propertiesRawUrl.value = ''
  propertiesProbeText.value = ''
  propertiesQrDataUrl.value = ''
  propertiesDialog.value = true
  if (file.type === 'folder') return

  propertiesLoading.value = true
  try {
    propertiesRawUrl.value = await filesStore.getRawUrl(file)
  } catch (error) {
    message.error(error instanceof Error ? error.message : '直链获取失败')
  } finally {
    propertiesLoading.value = false
  }
}

function openPropertiesRawUrl() {
  if (propertiesRawUrl.value) window.open(propertiesRawUrl.value, '_blank')
}

async function probePropertiesRawUrl() {
  if (!propertiesRawUrl.value) return
  propertiesProbeLoading.value = true
  propertiesProbeText.value = ''
  try {
    const result = await probeUrl(propertiesRawUrl.value)
    const sizeText = result.contentLength ? `，大小 ${formatBytes(result.contentLength)}` : ''
    const typeText = result.contentType ? `，类型 ${result.contentType}` : ''
    propertiesProbeText.value = result.ok ? `可访问，HTTP ${result.status}${typeText}${sizeText}` : `不可访问，HTTP ${result.status}`
  } catch (error) {
    propertiesProbeText.value = error instanceof Error ? error.message : '直链校验失败'
  } finally {
    propertiesProbeLoading.value = false
  }
}

async function generatePropertiesQr() {
  if (!propertiesRawUrl.value) return
  propertiesQrLoading.value = true
  try {
    propertiesQrDataUrl.value = await QRCode.toDataURL(propertiesRawUrl.value, {
      margin: 1,
      width: 180,
      errorCorrectionLevel: 'M'
    })
  } catch (error) {
    message.error(error instanceof Error ? error.message : '二维码生成失败')
  } finally {
    propertiesQrLoading.value = false
  }
}

async function openCloudDownload() {
  cloudUrls.value = ''
  cloudTargetPath.value = filesStore.currentPath
  cloudDownloadDialog.value = true
  try {
    const tools = await fsApi.offlineDownloadTools()
    if (tools.length) {
      cloudTools.value = tools
      const aria2 = tools.find((tool) => /aria2/i.test(tool))
      cloudTool.value = aria2 ?? (tools.includes('SimpleHttp') ? 'SimpleHttp' : tools[0])
    }
  } catch {
    cloudTools.value = ['SimpleHttp']
    cloudTool.value = 'SimpleHttp'
  }
}

async function submitCloudDownload() {
  const urls = cloudUrls.value
    .split(/\r?\n/)
    .map((url) => url.trim())
    .filter(Boolean)
  const target = cloudTargetPath.value.trim()

  if (!urls.length) {
    message.warning('请输入下载地址')
    return
  }
  if (!target) {
    message.warning('请选择保存目录')
    return
  }

  const task = tasksStore.addTask('download', `云端下载 ${urls.length} 个链接`, target)
  cloudSubmitting.value = true
  try {
    tasksStore.updateTask(task.id, { status: 'running', progress: 10 })
    await fsApi.offlineDownload({
      path: target,
      urls,
      tool: cloudTool.value,
      delete_policy: 'delete_on_upload_succeed'
    })
    tasksStore.updateTask(task.id, { status: 'success', progress: 100 })
    message.success('云端下载已提交')
    cloudDownloadDialog.value = false
    historyStore.add('download', target)
    await syncOfflineDownloadTasks()
  } catch (error) {
    tasksStore.updateTask(task.id, { status: 'failed' })
    message.error(error instanceof Error ? error.message : '云端下载提交失败')
  } finally {
    cloudSubmitting.value = false
  }
}

async function submitMkdir() {
  const name = folderName.value.trim()
  if (!name) return
  await filesStore.mkdir(name)
  message.success('文件夹已创建')
  mkdirDialog.value = false
}

function startRename(file: ExplorerFileItem) {
  activeFile.value = file
  renameValue.value = file.name
  renameDialog.value = true
}

async function submitRename() {
  const file = activeFile.value
  const name = renameValue.value.trim()
  if (!file || !name || name === file.name) return
  await filesStore.rename(file, name)
  message.success('已重命名')
  renameDialog.value = false
}

function confirmDelete(file?: ExplorerFileItem) {
  if (file) filesStore.selectedPaths = [file.path]
  const count = filesStore.selectedPaths.length
  if (!count) return

  dialog.warning({
    title: '删除项目',
    content: `确认删除选中的 ${count} 个项目？`,
    positiveText: '删除',
    negativeText: '取消',
    onPositiveClick: async () => {
      await filesStore.removeSelected()
      message.success('已删除')
    }
  })
}

function openTransfer(mode: TransferMode) {
  transferMode.value = mode
  destinationPath.value = filesStore.currentPath
  transferDialog.value = true
  resetTransferTree()
  loadTransferFolders(destinationPath.value)
}

async function loadTransferFolders(path: string) {
  const target = path.trim() || '/'
  destinationPath.value = target
  transferFoldersLoading.value = true
  try {
    const response = await fsApi.list({ path: target, page: 1, per_page: 500, refresh: false })
    transferFolders.value = (response.content ?? [])
      .filter((item) => item.is_dir)
      .map((item) => ({ name: item.name, path: joinPath(target, item.name) }))
  } catch (error) {
    transferFolders.value = []
    message.error(error instanceof Error ? error.message : '目录读取失败')
  } finally {
    transferFoldersLoading.value = false
  }
}

function resetTransferTree() {
  const storages = storageStore.storages.length
    ? storageStore.storages
    : storageStore.activeStorage
      ? [storageStore.activeStorage]
      : []
  transferTreeData.value = storages.map((storage) => ({
    label: storage.name,
    key: storage.mountPath,
    isLeaf: false
  }))
}

async function loadTransferTreeNode(node: TreeOption) {
  const path = String(node.key ?? '')
  if (!path) return
  const response = await fsApi.list({ path, page: 1, per_page: 500, refresh: false })
  node.children = (response.content ?? [])
    .filter((item) => item.is_dir)
    .map((item) => ({
      label: item.name,
      key: joinPath(path, item.name),
      isLeaf: false
    }))
}

function handleTransferTreeSelect(keys: Array<string | number>) {
  const key = keys[0]
  if (key === undefined) return
  loadTransferFolders(String(key))
}

async function submitTransfer() {
  const target = destinationPath.value.trim()
  if (!target) return

  if (transferMode.value === 'copy') {
    await filesStore.copySelected(target)
    message.success('已提交复制')
  } else {
    await filesStore.moveSelected(target)
    message.success('已移动')
  }

  transferDialog.value = false
}

async function pickFiles() {
  try {
    const selections = await selectUploadFiles(false)
    await uploadLocalSelections(selections)
  } catch (error) {
    if (String(error).includes('not implemented') || String(error).includes('__TAURI__')) {
      fileInput.value?.click()
      return
    }
    message.error(error instanceof Error ? error.message : '无法打开文件选择器')
  }
}

async function pickDirectory() {
  try {
    const selections = await selectUploadFiles(true)
    await uploadLocalSelections(selections)
  } catch (error) {
    if (String(error).includes('not implemented') || String(error).includes('__TAURI__')) {
      directoryInput.value?.click()
      return
    }
    message.error(error instanceof Error ? error.message : '无法打开目录选择器')
  }
}

async function handleUpload(event: Event) {
  const input = event.target as HTMLInputElement
  const pickedFiles = Array.from(input.files ?? [])
  input.value = ''
  await uploadFiles(pickedFiles)
}

async function handleDirectoryUpload(event: Event) {
  const input = event.target as HTMLInputElement
  const pickedFiles = Array.from(input.files ?? [])
  input.value = ''
  await uploadFiles(pickedFiles, true)
}

async function handleDrop(event: DragEvent) {
  dragOver.value = false
  await uploadFiles(Array.from(event.dataTransfer?.files ?? []))
}

async function uploadLocalPaths(paths: string[]) {
  if (!paths.length) return
  try {
    await uploadLocalSelections(await expandUploadPaths(paths))
  } catch (error) {
    message.error(error instanceof Error ? error.message : '无法读取拖拽文件')
  }
}

async function uploadLocalSelections(selections: LocalUploadSelection[]) {
  if (!filesStore.currentPath) {
    message.warning('请先连接 OpenList 并选择上传目录')
    return
  }
  if (!selections.length) return

  const token = await tokenVault.getToken(settingsStore.activeInstanceId)
  if (!token) {
    message.warning('请先连接 OpenList')
    return
  }

  message.info(`${selections.length} 个文件已加入上传列表`)
  const ensuredDirs = new Set<string>()
  async function ensureUploadParent(path: string) {
    const targetDir = dirname(path)
    const root = filesStore.currentPath.replace(/\/+$/, '')
    if (!targetDir || targetDir === filesStore.currentPath || ensuredDirs.has(targetDir)) return
    const relativeParts = targetDir.startsWith(root)
      ? targetDir.slice(root.length).split('/').filter(Boolean)
      : targetDir.split('/').filter(Boolean)
    let current = root || '/'
    for (const part of relativeParts) {
      current = joinPath(current, part)
      if (ensuredDirs.has(current)) continue
      try {
        await fsApi.mkdir(current)
      } catch {
        // OpenList returns an error when the directory already exists on some drivers.
      }
      ensuredDirs.add(current)
    }
  }

  const uploads: UploadQueueItem[] = selections.map((selection) => {
    const remotePath = joinPath(filesStore.currentPath, selection.relativePath)
    return {
      selection,
      task: tasksStore.addTask('upload', selection.relativePath, remotePath)
    }
  })

  await runLimited(uploads, settingsStore.uploadThreads, async ({ selection, task }) => {
    const canRun = await waitForRunnableTask(task.id)
    if (!canRun) return
    try {
      tasksStore.updateTask(task.id, { status: 'running', progress: 1, localPath: selection.path })
      await ensureUploadParent(task.path)
      await uploadWithEngine(task.id, settingsStore.serverUrl, token, selection.path, task.path)
      if (tasksStore.taskById(task.id)?.status === 'canceled') return
      tasksStore.updateTask(task.id, { status: 'success', progress: 100, localPath: selection.path })
      historyStore.add('upload', task.path)
    } catch (error) {
      if (tasksStore.taskById(task.id)?.status === 'canceled') return
      const text = error instanceof Error ? error.message : '上传失败'
      tasksStore.updateTask(task.id, {
        status: 'failed',
        message: `${text}。可重试该任务；当前 OpenList 上传接口未确认支持字节级断点续传。`
      })
      message.error(text)
    }
  })

  await filesStore.refresh()
}

async function uploadFiles(pickedFiles: File[], preserveRelativePath = false) {
  if (!filesStore.currentPath) {
    message.warning('请先连接 OpenList 并选择上传目录')
    return
  }
  if (!pickedFiles.length) return

  message.info(`${pickedFiles.length} 个文件已加入上传列表`)
  const ensuredDirs = new Set<string>()
  async function ensureUploadParent(path: string) {
    const targetDir = dirname(path)
    const root = filesStore.currentPath.replace(/\/+$/, '')
    if (!targetDir || targetDir === filesStore.currentPath || ensuredDirs.has(targetDir)) return
    const relativeParts = targetDir.startsWith(root)
      ? targetDir.slice(root.length).split('/').filter(Boolean)
      : targetDir.split('/').filter(Boolean)
    let current = root || '/'
    for (const part of relativeParts) {
      current = joinPath(current, part)
      if (ensuredDirs.has(current)) continue
      try {
        await fsApi.mkdir(current)
      } catch {
        // Existing directories are fine; OpenList returns an error for some drivers.
      }
      ensuredDirs.add(current)
    }
  }

  const uploads = pickedFiles.map((file) => ({
    file,
    task: tasksStore.addTask(
      'upload',
      preserveRelativePath ? ((file as File & { webkitRelativePath?: string }).webkitRelativePath || file.name) : file.name,
      joinPath(
        filesStore.currentPath,
        preserveRelativePath ? ((file as File & { webkitRelativePath?: string }).webkitRelativePath || file.name) : file.name
      )
    )
  }))
  await runLimited(uploads, settingsStore.uploadThreads, async ({ file, task }) => {
    const canRun = await waitForRunnableTask(task.id)
    if (!canRun) return
    try {
      tasksStore.updateTask(task.id, { status: 'running' })
      if (preserveRelativePath) await ensureUploadParent(task.path)
      await fsApi.upload(task.path, file, (progress) => tasksStore.updateTask(task.id, { progress }))
      if (tasksStore.taskById(task.id)?.status === 'canceled') return
      tasksStore.updateTask(task.id, { status: 'success', progress: 100 })
      historyStore.add('upload', task.path)
    } catch (error) {
      const text = error instanceof Error ? error.message : '上传失败'
      tasksStore.updateTask(task.id, {
        status: 'failed',
        message: `${text}。可重新上传；浏览器文件对象关闭后无法做字节级断点续传。`
      })
      message.error(text)
    }
  })

  if (pickedFiles.length) await filesStore.refresh()
}

onMounted(async () => {
  try {
    unlistenDragDrop = await getCurrentWindow().onDragDropEvent(async (event) => {
      if (event.payload.type === 'enter' || event.payload.type === 'over') {
        dragOver.value = true
        return
      }
      if (event.payload.type === 'leave') {
        dragOver.value = false
        return
      }
      dragOver.value = false
      await uploadLocalPaths(event.payload.paths)
    })
  } catch {
    // Browser preview does not expose Tauri drag/drop events.
  }

  if (storageStore.activeStorage) {
    filesStore.load(storageStore.activeStorage.mountPath)
  } else {
    filesStore.lastError = '请先设置 OpenList 连接'
  }
})

onBeforeUnmount(() => {
  unlistenDragDrop?.()
})
</script>
