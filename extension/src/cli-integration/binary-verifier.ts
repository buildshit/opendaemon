import * as fs from 'fs/promises';

/**
 * Result of binary verification
 */
export interface VerificationResult {
    exists: boolean;
    hasPermissions: boolean;
    error?: string;
}

/**
 * Verifies that a binary file exists and has the correct permissions
 * @param binaryPath - Absolute path to the binary file
 * @returns Verification result with exists, hasPermissions, and optional error
 */
export async function verifyBinary(binaryPath: string): Promise<VerificationResult> {
    try {
        // Check if file exists
        await fs.access(binaryPath, fs.constants.F_OK);
        
        // On Unix-like systems, check execute permissions
        const isWindows = process.platform === 'win32';
        
        if (!isWindows) {
            try {
                await fs.access(binaryPath, fs.constants.X_OK);
                return {
                    exists: true,
                    hasPermissions: true
                };
            } catch (permError) {
                return {
                    exists: true,
                    hasPermissions: false,
                    error: 'Binary lacks execute permissions'
                };
            }
        }
        
        // On Windows, execute permissions are not applicable
        return {
            exists: true,
            hasPermissions: true
        };
        
    } catch (error) {
        const errorMessage = `Binary not found at: ${binaryPath}`;
        return {
            exists: false,
            hasPermissions: false,
            error: errorMessage
        };
    }
}

/**
 * Attempts to fix execute permissions on a binary file (Unix-like systems only)
 * @param binaryPath - Absolute path to the binary file
 * @returns True if permissions were successfully set, false otherwise
 */
export async function fixPermissions(binaryPath: string): Promise<boolean> {
    // Only applicable on Unix-like systems
    if (process.platform === 'win32') {
        return true;
    }
    
    try {
        await fs.chmod(binaryPath, 0o755);
        return true;
    } catch (error) {
        return false;
    }
}
