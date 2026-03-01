import * as assert from 'assert';
import * as fc from 'fast-check';
import * as vscode from 'vscode';
import { OpenDaemonTerminalProfileProvider } from '../../cli-integration/terminal-profile-provider';

suite('Terminal Profile Provider Property Tests', () => {
    /**
     * Property 1: Profile Provider Returns Valid Terminal Options
     * Feature: automatic-terminal-profile-registration
     * Validates: Requirements 1.4, 1.5
     * 
     * For any CLI binary path, the returned terminal profile should include 
     * a valid PATH environment variable that contains the CLI binary directory.
     */
    test('Property 1: Profile Provider Returns Valid Terminal Options', () => {
        fc.assert(
            fc.property(
                // Generate random valid directory paths
                fc.array(fc.stringMatching(/^[a-zA-Z0-9_-]+$/), { minLength: 1, maxLength: 5 }).map(parts => {
                    // Create a valid path from parts
                    if (process.platform === 'win32') {
                        return `C:\\${parts.join('\\')}\\bin`;
                    } else {
                        return `/${parts.join('/')}/bin`;
                    }
                }),
                (binDir) => {
                    // Create provider with the generated bin directory
                    const provider = new OpenDaemonTerminalProfileProvider(binDir);
                    
                    // Get terminal profile
                    const profile = provider.provideTerminalProfile(
                        new vscode.CancellationTokenSource().token
                    ) as vscode.TerminalProfile;
                    
                    // Verify profile is returned
                    assert.ok(profile, 'Profile should be returned');
                    
                    // Verify profile has options
                    assert.ok(profile.options, 'Profile should have options');
                    
                    // Verify env is set
                    const options = profile.options as vscode.TerminalOptions;
                    assert.ok(options.env, 'Profile options should have env');
                    
                    // Verify PATH contains the bin directory
                    const path = options.env!.PATH || options.env!.Path;
                    assert.ok(path, 'PATH should be set in env');
                    assert.ok(
                        path!.includes(binDir),
                        `PATH should contain bin directory: ${binDir}, but got: ${path}`
                    );
                }
            ),
            { numRuns: 100 }
        );
    });

    /**
     * Property 3: Platform-Specific PATH Formatting
     * Feature: automatic-terminal-profile-registration
     * Validates: Requirements 3.1, 3.2, 3.3, 3.4
     * 
     * For any platform (Windows, macOS, Linux), the PATH environment variable 
     * should use the correct platform-specific separator.
     */
    test('Property 3: Platform-Specific PATH Formatting', () => {
        fc.assert(
            fc.property(
                // Generate random bin directories
                fc.array(fc.stringMatching(/^[a-zA-Z0-9_-]+$/), { minLength: 1, maxLength: 3 }).map(parts => {
                    if (process.platform === 'win32') {
                        return `C:\\${parts.join('\\')}`;
                    } else {
                        return `/${parts.join('/')}`;
                    }
                }),
                // Generate random existing PATH values
                fc.array(fc.stringMatching(/^[a-zA-Z0-9_\/-]+$/), { minLength: 0, maxLength: 3 }).map(parts => {
                    const separator = process.platform === 'win32' ? ';' : ':';
                    return parts.join(separator);
                }),
                (binDir, existingPath) => {
                    // Set up environment
                    const originalPath = process.env.PATH;
                    process.env.PATH = existingPath;
                    
                    try {
                        // Create provider
                        const provider = new OpenDaemonTerminalProfileProvider(binDir);
                        
                        // Get terminal profile
                        const profile = provider.provideTerminalProfile(
                            new vscode.CancellationTokenSource().token
                        ) as vscode.TerminalProfile;
                        
                        // Get PATH from profile
                        const options = profile.options as vscode.TerminalOptions;
                        const path = options.env!.PATH || options.env!.Path;
                        
                        // Determine expected separator
                        const expectedSeparator = process.platform === 'win32' ? ';' : ':';
                        
                        // If there was an existing path, verify separator is used
                        if (existingPath) {
                            assert.ok(
                                path!.includes(expectedSeparator),
                                `PATH should use platform separator '${expectedSeparator}': ${path}`
                            );
                        }
                        
                        // Verify bin directory is at the start
                        assert.ok(
                            path!.startsWith(binDir),
                            `PATH should start with bin directory: ${binDir}, but got: ${path}`
                        );
                    } finally {
                        // Restore original PATH
                        if (originalPath !== undefined) {
                            process.env.PATH = originalPath;
                        } else {
                            delete process.env.PATH;
                        }
                    }
                }
            ),
            { numRuns: 100 }
        );
    });
});
