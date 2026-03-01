/**
 * Platform detection module for CLI integration
 * Identifies the current operating system and architecture
 */

export interface PlatformInfo {
  os: 'win32' | 'darwin' | 'linux';
  arch: 'x64' | 'arm64';
}

/**
 * Detects the current platform (OS and architecture)
 * @returns PlatformInfo object with os and arch
 * @throws Error if platform is unsupported
 */
export function detectPlatform(): PlatformInfo {
  const platform = process.platform;
  const arch = process.arch;

  // Validate OS
  if (platform !== 'win32' && platform !== 'darwin' && platform !== 'linux') {
    throw new Error(
      `Unsupported operating system: ${platform}. ` +
      `Supported platforms are: win32 (Windows), darwin (macOS), linux (Linux)`
    );
  }

  // Validate architecture
  if (arch !== 'x64' && arch !== 'arm64') {
    throw new Error(
      `Unsupported architecture: ${arch}. ` +
      `Supported architectures are: x64, arm64`
    );
  }

  return {
    os: platform as 'win32' | 'darwin' | 'linux',
    arch: arch as 'x64' | 'arm64'
  };
}
