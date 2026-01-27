/**
 * Binary resolution module for CLI integration
 * Determines the correct CLI binary path for the current platform
 */

import * as path from 'path';
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
 * @returns BinaryInfo object with name, fullPath, and binDir
 */
export function resolveBinary(extensionPath: string, platform: PlatformInfo): BinaryInfo {
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
