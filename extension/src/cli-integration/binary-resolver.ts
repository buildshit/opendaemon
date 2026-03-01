/**
 * Binary resolution module for CLI integration
 * Determines the correct CLI binary path for the current platform
 */

import * as path from 'path';
import * as fs from 'fs';
import { PlatformInfo } from './platform-detector';

export interface BinaryInfo {
  name: string;        // e.g., "dmn-win32-x64.exe"
  fullPath: string;    // e.g., "/path/to/extension/bin/dmn-win32-x64.exe"
  binDir: string;      // e.g., "/path/to/extension/bin"
}

/**
 * Resolves the binary path for the given platform
 * @param extensionPath - The absolute path to the extension directory
 * @param platform - The platform information (os and arch)
 * @param workspaceRoot - Optional workspace root for preferring local builds
 * @returns BinaryInfo object with name, fullPath, and binDir
 */
export function resolveBinary(
  extensionPath: string,
  platform: PlatformInfo,
  workspaceRoot?: string
): BinaryInfo {
  // Prefer locally built workspace binary when available so terminal CLI
  // matches the daemon binary chosen by DaemonManager.
  const localBinary = resolveLocalBinary(platform, workspaceRoot);
  if (localBinary) {
    return localBinary;
  }

  // Construct binary name based on platform
  const binaryName = constructBinaryName(platform);
  
  // Build full path using path.join
  const binDir = path.join(extensionPath, 'bin');
  const fullPath = path.join(binDir, binaryName);
  
  return {
    name: binaryName,
    fullPath: fullPath,
    binDir: binDir
  };
}

function resolveLocalBinary(platform: PlatformInfo, workspaceRoot?: string): BinaryInfo | null {
  if (!workspaceRoot) {
    return null;
  }

  const executableName = platform.os === 'win32' ? 'dmn.exe' : 'dmn';
  const candidates = [
    path.join(workspaceRoot, 'target', 'release', executableName),
    path.join(workspaceRoot, 'target', 'debug', executableName),
    path.join(workspaceRoot, 'target', 'build-current', 'release', executableName),
    path.join(workspaceRoot, 'target', 'build-current', 'debug', executableName)
  ];
  const selected = pickNewestBinary(candidates);
  if (selected) {
    return {
      name: executableName,
      fullPath: selected,
      binDir: path.dirname(selected)
    };
  }

  return null;
}

function pickNewestBinary(candidates: string[]): string | null {
  let bestPath: string | null = null;
  let bestMtime = -1;

  for (const candidate of candidates) {
    if (!fs.existsSync(candidate)) {
      continue;
    }

    const stat = fs.statSync(candidate);
    if (stat.mtimeMs > bestMtime) {
      bestMtime = stat.mtimeMs;
      bestPath = candidate;
    }
  }

  return bestPath;
}

/**
 * Constructs the binary name based on platform
 * @param platform - The platform information
 * @returns Binary name (e.g., "dmn-win32-x64.exe")
 */
function constructBinaryName(platform: PlatformInfo): string {
  const { os, arch } = platform;
  
  // Binary naming convention: dmn-{os}-{arch}[.exe]
  const baseName = `dmn-${os}-${arch}`;
  
  // Add .exe extension for Windows
  if (os === 'win32') {
    return `${baseName}.exe`;
  }
  
  return baseName;
}
