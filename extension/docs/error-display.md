# Error Display System

## Overview

The Error Display System provides comprehensive error handling and user feedback for the OpenDaemon VS Code extension. It displays error notifications, logs detailed error information to an output panel, and provides actionable error messages with context-appropriate actions.

## Features

### 1. Error Notifications

The system displays errors using VS Code's notification system with three severity levels:

- **Critical**: Red error notifications for configuration, graph, process, and service errors
- **Warning**: Yellow warning notifications for ready check failures and orchestrator issues
- **Info**: Blue information notifications for MCP, RPC, IO, and JSON errors

### 2. Error Output Panel

All errors are logged to a dedicated "OpenDaemon Errors" output panel with:

- Timestamp for each error
- Error category (CONFIG, GRAPH, PROCESS, etc.)
- Detailed error message
- Service name (if applicable)
- Exit code (if applicable)
- Additional details (if available)

### 3. Actionable Error Messages

Each error notification includes contextual actions:

- **Config/Graph Errors**: "Open Config" button to edit dmn.json
- **Process/Service Errors**: "Show Logs" button to view service logs
- **Ready Errors**: "Show Logs" button to debug readiness issues
- **Orchestrator Errors**: "Reload Window" button to restart the extension
- **All Errors**: "Show Details" button to open the error output panel

### 4. Error History

The system maintains a history of up to 100 recent errors, which can be:

- Viewed via the error output panel
- Cleared using the "Clear Errors" command
- Accessed programmatically for debugging

## Architecture

### Components

#### ErrorDisplayManager

The main class that manages error display functionality:

```typescript
class ErrorDisplayManager {
    constructor(actionHandlers: ErrorActionHandlers);
    
    // Display methods
    displayError(error: ErrorInfo): Promise<void>;
    displayServiceFailure(service: string, errorMessage: string, exitCode?: number): Promise<void>;
    displayConfigError(message: string, details?: string): Promise<void>;
    displayGraphError(message: string): Promise<void>;
    displayProcessError(service: string, message: string): Promise<void>;
    displayReadyError(service: string, message: string): Promise<void>;
    
    // Output panel methods
    showOutputPanel(): void;
    clearOutputPanel(): void;
    
    // History methods
    getErrorHistory(): ErrorInfo[];
    clearHistory(): void;
    
    // Cleanup
    dispose(): void;
}
```

#### ErrorInfo Interface

Represents error information:

```typescript
interface ErrorInfo {
    message: string;
    category: ErrorCategory;
    severity?: ErrorSeverity;
    service?: string;
    exitCode?: number;
    timestamp?: Date;
    details?: string;
}
```

#### ErrorCategory Enum

Maps to Rust error types:

```typescript
enum ErrorCategory {
    CONFIG = 'CONFIG',
    GRAPH = 'GRAPH',
    PROCESS = 'PROCESS',
    READY = 'READY',
    ORCHESTRATOR = 'ORCHESTRATOR',
    MCP = 'MCP',
    RPC = 'RPC',
    IO = 'IO',
    JSON = 'JSON',
    SERVICE = 'SERVICE'
}
```

#### ErrorSeverity Enum

Defines error severity levels:

```typescript
enum ErrorSeverity {
    Critical = 'critical',
    Warning = 'warning',
    Info = 'info'
}
```

## Integration

### Extension Integration

The ErrorDisplayManager is initialized in the extension activation:

```typescript
errorDisplayManager = new ErrorDisplayManager({
    openConfig: async () => await openDmnConfig(),
    showLogs: (service?: string) => {
        if (service && logManager) {
            logManager.showLogs(service);
        } else {
            errorDisplayManager?.showOutputPanel();
        }
    },
    reload: () => {
        vscode.commands.executeCommand('workbench.action.reloadWindow');
    },
    retry: async () => {
        await handleConfigChanged();
    }
});
```

### Error Handling in Extension Code

Use the appropriate display method based on error type:

```typescript
// Configuration errors
await errorDisplayManager.displayConfigError(
    'Missing required field: version',
    'Field "version" is required in dmn.json'
);

// Service failures
await errorDisplayManager.displayServiceFailure(
    'backend',
    'Connection refused',
    1
);

// Generic errors
await errorDisplayManager.displayError({
    message: 'Failed to start daemon',
    category: ErrorCategory.ORCHESTRATOR,
    details: err.stack
});
```

### Daemon Notification Handling

The extension automatically handles error notifications from the Rust daemon:

```typescript
if (method === 'error') {
    const { message, category } = params as {
        message: string;
        category: string;
    };
    
    if (errorDisplayManager) {
        errorDisplayManager.displayError({
            message,
            category: category as ErrorCategory
        });
    }
}
```

## Commands

### opendaemon.showErrors

Shows the error output panel with all logged errors.

**Usage**: Command Palette → "OpenDaemon: Show Errors"

### opendaemon.clearErrors

Clears the error history and output panel.

**Usage**: Command Palette → "OpenDaemon: Clear Errors"

## Error Flow

### 1. Error Occurs

An error occurs in the Rust daemon or extension code.

### 2. Error Display

The error is passed to `ErrorDisplayManager.displayError()` or a specialized method.

### 3. Severity Determination

If severity is not provided, it's determined based on the error category:

- CONFIG, GRAPH, PROCESS, SERVICE → Critical
- READY, ORCHESTRATOR → Warning
- MCP, RPC, IO, JSON → Info

### 4. Logging

The error is logged to the output panel with full details.

### 5. Notification

A VS Code notification is shown with appropriate actions.

### 6. History

The error is added to the error history (max 100 entries).

### 7. User Action

User can click on action buttons to:

- Open the configuration file
- View service logs
- Show error details
- Reload the window
- Retry the operation

## Testing

### Unit Tests

Located in `extension/src/test/suite/error-display.test.ts`:

- Error history management
- Severity determination
- Error display methods
- History size limits
- Error field handling

### Integration Tests

Test error display in real scenarios:

- Configuration parsing errors
- Service startup failures
- Dependency graph cycles
- Ready check timeouts
- RPC communication errors

## Best Practices

### 1. Use Specific Display Methods

Prefer specific methods over generic `displayError()`:

```typescript
// Good
await errorDisplayManager.displayConfigError('Invalid JSON');

// Less specific
await errorDisplayManager.displayError({
    message: 'Invalid JSON',
    category: ErrorCategory.CONFIG
});
```

### 2. Include Context

Always include relevant context in error messages:

```typescript
// Good
await errorDisplayManager.displayProcessError(
    'backend',
    'Failed to spawn: command not found'
);

// Too vague
await errorDisplayManager.displayError({
    message: 'Failed',
    category: ErrorCategory.PROCESS
});
```

### 3. Provide Details

Include stack traces or additional information in the details field:

```typescript
await errorDisplayManager.displayError({
    message: 'Unexpected error',
    category: ErrorCategory.ORCHESTRATOR,
    details: err instanceof Error ? err.stack : String(err)
});
```

### 4. Handle Errors Gracefully

Always check if errorDisplayManager exists before using it:

```typescript
if (errorDisplayManager) {
    await errorDisplayManager.displayError(error);
} else {
    // Fallback to basic VS Code notification
    vscode.window.showErrorMessage(error.message);
}
```

## Future Enhancements

1. **Error Filtering**: Filter errors by category or severity in the output panel
2. **Error Search**: Search through error history
3. **Error Export**: Export error history to a file for debugging
4. **Error Statistics**: Show error counts by category
5. **Error Grouping**: Group similar errors together
6. **Error Notifications**: Configurable notification preferences
7. **Error Recovery**: Automatic retry for recoverable errors
