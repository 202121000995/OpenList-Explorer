import { useMessage } from 'naive-ui'

export function useClipboardAction() {
  const message = useMessage()

  async function copyText(text: string, successText = '已复制') {
    await navigator.clipboard.writeText(text)
    message.success(successText)
  }

  return { copyText }
}
