import { clsx, type ClassValue } from "clsx"
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

/**
 * Convert a local file path to a Tauri asset protocol URL.
 * In non-Tauri environments (dev browser), returns the path as-is.
 */
export function assetUrl(filePath: string | null | undefined): string | undefined {
  if (!filePath) return undefined
  if (filePath.startsWith('http://') || filePath.startsWith('https://') || filePath.startsWith('asset://')) {
    return filePath
  }
  // Tauri 2 asset protocol: https://asset.localhost/{encoded_path}
  if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
    return `https://asset.localhost/${encodeURIComponent(filePath)}`
  }
  return filePath
}
