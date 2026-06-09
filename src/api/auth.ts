import { openListHttp } from './http'

export interface LoginPayload {
  username: string
  password: string
}

export interface LoginResponse {
  token?: string
}

export const authApi = {
  login(payload: LoginPayload) {
    return openListHttp.post<unknown, LoginResponse>('/api/auth/login', payload)
  }
}
