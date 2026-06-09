export type TransferType = 'upload' | 'download'
export type TransferStatus = 'waiting' | 'running' | 'success' | 'failed' | 'paused' | 'canceled'

export interface TransferTask {
  id: string
  type: TransferType
  status: TransferStatus
  progress: number
  speed: number
  path: string
  localPath?: string
  name: string
  createdAt: number
}

export const taskStatusLabel: Record<TransferStatus, string> = {
  waiting: '等待',
  running: '进行中',
  success: '成功',
  failed: '失败',
  paused: '暂停',
  canceled: '已取消'
}
