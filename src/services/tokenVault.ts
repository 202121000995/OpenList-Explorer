import { invoke } from '@tauri-apps/api/core'

const legacyTokenKey = 'openlist.authToken'

function readLegacyToken() {
  try {
    return localStorage.getItem(legacyTokenKey) ?? ''
  } catch {
    return ''
  }
}

function clearLegacyToken() {
  try {
    localStorage.removeItem(legacyTokenKey)
  } catch {
    // Ignore legacy cleanup failures.
  }
}

async function readCredentialToken(instanceId: string) {
  try {
    return (await invoke<string | null>('read_openlist_token', { instanceId })) ?? ''
  } catch {
    return ''
  }
}

const sessionTokens = new Map<string, string>()

async function persistToken(instanceId: string, token: string) {
  clearLegacyToken()

  try {
    if (token) await invoke('save_openlist_token', { instanceId, token })
    else await invoke('clear_openlist_token', { instanceId })
  } catch {
    // Keep the in-memory token in non-Tauri development environments.
  }
}

export const tokenVault = {
  async getToken(instanceId: string) {
    const cached = sessionTokens.get(instanceId)
    if (cached) return cached

    const credentialToken = await readCredentialToken(instanceId)
    if (credentialToken) {
      sessionTokens.set(instanceId, credentialToken)
      return credentialToken
    }

    const legacyToken = readLegacyToken()
    if (legacyToken) {
      const token = legacyToken.trim()
      sessionTokens.set(instanceId, token)
      await persistToken(instanceId, token)
    }

    return sessionTokens.get(instanceId) ?? ''
  },

  async setToken(instanceId: string, token: string) {
    const trimmed = token.trim()
    if (trimmed) sessionTokens.set(instanceId, trimmed)
    else sessionTokens.delete(instanceId)
    await persistToken(instanceId, trimmed)
  },

  async clearToken(instanceId: string) {
    sessionTokens.delete(instanceId)
    clearLegacyToken()

    try {
      await invoke('clear_openlist_token', { instanceId })
    } catch {
      // Nothing to clear in non-Tauri development environments.
    }
  },

  async hasStoredToken(instanceId: string) {
    return Boolean(sessionTokens.get(instanceId) || (await readCredentialToken(instanceId)) || readLegacyToken())
  }
}

export const tokenVaultNotes = [
  'Desktop builds store the reusable API token in Windows Credential Manager.',
  'Development browser sessions keep tokens in memory because Tauri commands are unavailable.'
]
