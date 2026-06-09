import axios from 'axios'
import { useSettingsStore } from '@/stores/settings'
import { tokenVault } from '@/services/tokenVault'

export const openListHttp = axios.create({
  timeout: 20000
})

openListHttp.interceptors.request.use(async (config) => {
  const settings = useSettingsStore()
  settings.ensureInstances()
  const token = await tokenVault.getToken(settings.activeInstanceId)
  config.baseURL = settings.serverUrl

  if (token) {
    config.headers.Authorization = token
  }

  return config
})

openListHttp.interceptors.response.use((response) => {
  const payload = response.data

  if (payload && typeof payload === 'object' && 'code' in payload) {
    if (payload.code !== 200) {
      throw new Error(payload.message || payload.msg || 'OpenList API request failed')
    }

    return payload.data
  }

  return payload
})
