export type TransferType = 'upload' | 'download'
export type TransferStatus = 'waiting' | 'running' | 'success' | 'failed' | 'paused' | 'canceled'
export type TransferSource = 'local' | 'openlist-offline'
export type TransferStage =
  | 'local'
  | 'queued'
  | 'downloading'
  | 'uploading'
  | 'completed'
  | 'failed'
  | 'paused'
  | 'canceled'
  | 'syncing'

export interface TransferTask {
  id: string
  type: TransferType
  status: TransferStatus
  progress: number
  speed: number
  path: string
  localPath?: string
  remoteId?: string
  remoteUrl?: string
  instanceId?: string
  source?: TransferSource
  stage?: TransferStage
  rawStatus?: string
  failureReason?: string
  completedDir?: string
  message?: string
  name: string
  createdAt: number
  updatedAt?: number
}

export const taskStatusLabel: Record<TransferStatus, string> = {
  waiting: '等待',
  running: '进行中',
  success: '成功',
  failed: '失败',
  paused: '暂停',
  canceled: '已取消'
}

export const taskStageLabel: Record<TransferStage, string> = {
  local: '本地传输',
  queued: '排队中',
  downloading: '下载中',
  uploading: '上传到网盘',
  completed: '已完成',
  failed: '失败',
  paused: '已暂停',
  canceled: '已取消',
  syncing: '同步中'
}
