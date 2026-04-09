import { clsx, type ClassValue } from "clsx"
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

/**
 * Convert a local file path to a Tauri asset protocol URL.
 * Uses Tauri's built-in convertFileSrc for correct platform-specific URLs.
 * In non-Tauri environments (dev browser), returns the path as-is.
 */
export function assetUrl(filePath: string | null | undefined): string | undefined {
  if (!filePath) return undefined
  if (filePath.startsWith('http://') || filePath.startsWith('https://') || filePath.startsWith('asset://')) {
    return filePath
  }
  // Tauri 2 asset protocol: delegates to __TAURI_INTERNALS__.convertFileSrc
  // which produces http://asset.localhost/{encoded_path} on Windows
  if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
    return (window as any).__TAURI_INTERNALS__.convertFileSrc(filePath, 'asset')
  }
  return filePath
}
