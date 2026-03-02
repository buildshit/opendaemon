// Simple test script to verify binary path selection logic
const path = require('path');
const fs = require('fs');

function getBinaryPath(extensionPath) {
    const platform = process.platform;
    const arch = process.arch;
    let binaryName;
    
    // Select platform-specific binary
    if (platform === 'win32') {
        binaryName = 'dmn-win32-x64.exe';
    } else if (platform === 'darwin') {
        // macOS - check architecture
        if (arch === 'arm64') {
            binaryName = 'dmn-darwin-arm64';
        } else {
            binaryName = 'dmn-darwin-x64';
        }
    } else if (platform === 'linux') {
        binaryName = arch === 'arm64' ? 'dmn-linux-arm64' : 'dmn-linux-x64';
    } else {
        throw new Error(`Unsupported platform: ${platform}`);
    }

    // Look for binary in extension's bin directory
    const binPath = path.join(extensionPath, 'bin', binaryName);
    
    return binPath;
}

// Test the function
const extensionPath = path.join(__dirname, '..', 'extension');
const binaryPath = getBinaryPath(extensionPath);

console.log('Platform:', process.platform);
console.log('Architecture:', process.arch);
console.log('Expected binary path:', binaryPath);
console.log('Binary exists:', fs.existsSync(binaryPath));

if (fs.existsSync(binaryPath)) {
    const stats = fs.statSync(binaryPath);
    console.log('Binary size:', stats.size, 'bytes');
    
    if (process.platform !== 'win32') {
        const isExecutable = (stats.mode & fs.constants.S_IXUSR) !== 0;
        console.log('Is executable:', isExecutable);
    }
}
