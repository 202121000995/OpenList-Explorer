export interface StorageEntry {
  id: string
  name: string
  driver: string
  mountPath: string
  color: string
  iconText: string
  logoKey?: string
  usedBytes?: number
  totalBytes?: number
}

export const defaultStorages: StorageEntry[] = []
