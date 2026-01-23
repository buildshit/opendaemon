# Design Document

## Overview

This design addresses the bugs preventing the OpenDaemon VS Code extension from properly displaying services and starting them. The root causes are:

1. **Service Discovery Issue**: The `loadServicesFromConfig()` function is called but the tree view may not be properly initialized or the services aren't being added correctly
2. **Timeout Configuration**: The default 30-second timeout in the Rust daemon may be insufficient for slow-starting services
3. **RPC Communication**: There may be timing issues between when the extension requests service status and when the daemon is ready to respond

## Architecture

### Component Interaction Flow

```
Extension Activation
    ↓
Find dmn.json
    ↓
Initialize Tree View (empty initially)
    ↓
Load Services from Config → Populate Tree View (NotStarted status)
    ↓
Start Daemon
    ↓
Initialize RPC Client
    ↓
Request Service Status from Daemon → Update Tree View with actual statuses
    ↓
Listen for Status Change Notifications → Update Tree View in real-time
```

### Key Components

1. **ServiceTreeDataProvider** (extension/src/tree-view.ts)
   - Manages the tree view state
   - Stores service information in a Map
   - Provides methods to update individual services or all services at once

2. **Extension Activation** (extension/src/extension.ts)
   - Orchestrates the initialization sequence
   - Calls `loadServicesFromConfig()` to populate tree view before daemon starts
   - Handles daemon startup and RPC initialization

3. **ReadyWatcher** (core/src/ready.rs)
   - Monitors service readiness conditions
   - Has configurable timeout (default 30 seconds)
   - Supports both log pattern matching and URL polling

4. **RPC Client** (extension/src/rpc-client.ts)
   - Handles JSON-RPC communication with daemon
   - Sends requests and receives responses/notifications

## Components and Interfaces

### 1. Service Loading Enhancement

**Problem**: Services aren't appearing in the tree view even though `loadServicesFromConfig()` is called.

**Solution**: Ensure the tree view is properly initialized and services are added before any daemon operations.

```typescript
// extension/src/extension.ts

async function loadServicesFromConfig(configPath: string): Promise<void> {
    if (!treeDataProvider) {
        console.error('Tree data provider not initialized');
        return;
    }

    try {
        const configContent = await fs.promises.readFile(configPath, 'utf-8');
        const config = JSON.parse(configContent) as { services?: Record<string, unknown> };

        if (config.services) {
            const services = Object.keys(config.services).map(name => ({
                name,
                status: ServiceStatus.NotStarted
            }));

            treeDataProvider.updateServices(services);
            console.log(`Loaded ${services.length} services from config: ${services.map(s => s.name).join(', ')}`);
        } else {
            console.warn('No services found in dmn.json');
            treeDataProvider.updateServices([]);
        }
    } catch (err) {
        console.error('Failed to load services from config:', err);
        
        if (errorDisplayManager) {
            await errorDisplayManager.displayError({
                message: `Failed to load services from dmn.json: ${err instanceof Error ? err.message : String(err)}`,
                category: ErrorCategory.CONFIG,
                details: err instanceof Error ? err.stack : undefined
            });
        }
    }
}
```

### 2. Timeout Configuration

**Problem**: Services timeout before they can complete their startup sequence.

**Solution**: Add configurable timeout support in dmn.json and increase default timeout.

```json
// dmn.json schema extension
{
  "services": {
    "database": {
      "command": "...",
      "ready_when": {
        "type": "log_contains",
        "pattern": "Database Ready",
        "timeout_seconds": 60  // Optional custom timeout
      }
    }
  }
}
```

```rust
// core/src/config.rs additions

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ReadyCondition {
    LogContains { 
        pattern: String,
        #[serde(default)]
        timeout_seconds: Option<u64>
    },
    UrlResponds { 
        url: String,
        #[serde(default)]
        timeout_seconds: Option<u64>
    },
}
```

```rust
// core/src/orchestrator.rs modifications

// Increase default timeout from 30 to 60 seconds
let ready_watcher = Arc::new(Mutex::new(ReadyWatcher::new(Duration::from_secs(60))));

// Use custom timeout if specified
let timeout = service_config.ready_when
    .as_ref()
    .and_then(|r| match r {
        ReadyCondition::LogContains { timeout_seconds, .. } => *timeout_seconds,
        ReadyCondition::UrlResponds { timeout_seconds, .. } => *timeout_seconds,
    })
    .map(Duration::from_secs)
    .unwrap_or(Duration::from_secs(60));
```

### 3. Command Palette Service Selection

**Problem**: "No services found" error when using command palette commands.

**Solution**: Ensure commands check tree view state and provide helpful error messages.

```typescript
// extension/src/commands.ts

private async getServiceItem(item?: ServiceTreeItem): Promise<ServiceTreeItem | undefined> {
    if (item) {
        return item;
    }

    const treeDataProvider = this.getTreeDataProvider();
    if (!treeDataProvider) {
        vscode.window.showErrorMessage(
            'OpenDaemon tree view not initialized. Please reload the window.',
            'Reload'
        ).then(selection => {
            if (selection === 'Reload') {
                vscode.commands.executeCommand('workbench.action.reloadWindow');
            }
        });
        return undefined;
    }

    const services = treeDataProvider.getAllServices();
    if (services.length === 0) {
        const configPath = fileWatcher?.getConfigPath();
        if (!configPath) {
            vscode.window.showErrorMessage(
                'No dmn.json file found in workspace. Would you like to create one?',
                'Create dmn.json'
            ).then(selection => {
                if (selection === 'Create dmn.json') {
                    vscode.commands.executeCommand('opendaemon.createConfig');
                }
            });
        } else {
            vscode.window.showErrorMessage(
                'No services found in dmn.json. Please add services to your configuration.',
                'Open dmn.json'
            ).then(selection => {
                if (selection === 'Open dmn.json') {
                    vscode.workspace.openTextDocument(configPath).then(doc => {
                        vscode.window.showTextDocument(doc);
                    });
                }
            });
        }
        return undefined;
    }

    const items = services.map(s => ({
        label: s.name,
        description: String(s.status),
        service: s
    }));

    const selected = await vscode.window.showQuickPick(items, {
        placeHolder: 'Select a service'
    });

    if (selected) {
        const s = selected.service;
        return new ServiceTreeItem(s.name, s.status, s.exitCode);
    }

    return undefined;
}
```

### 4. Initialization Sequence Improvement

**Problem**: Race condition between tree view initialization and daemon startup.

**Solution**: Ensure proper sequencing with clear logging.

```typescript
// extension/src/extension.ts

async function initializeDaemon(dmnConfigPath: string): Promise<void> {
    console.log(`[OpenDaemon] Initializing with config: ${dmnConfigPath}`);
    
    vscode.window.showInformationMessage(
        `OpenDaemon: Found configuration at ${path.basename(dmnConfigPath)}`
    );

    // Step 1: Load services from config file FIRST
    // This populates the tree view immediately so users see services
    console.log('[OpenDaemon] Step 1: Loading services from config...');
    await loadServicesFromConfig(dmnConfigPath);
    
    const treeServices = treeDataProvider?.getAllServices() || [];
    console.log(`[OpenDaemon] Tree view now has ${treeServices.length} services: ${treeServices.map(s => s.name).join(', ')}`);

    // Step 2: Start file watcher
    console.log('[OpenDaemon] Step 2: Starting file watcher...');
    if (fileWatcher) {
        fileWatcher.start(dmnConfigPath);
    }

    // Step 3: Initialize daemon manager
    console.log('[OpenDaemon] Step 3: Initializing daemon manager...');
    daemonManager = new DaemonManager(
        extensionContext!,
        (data) => handleDaemonStdout(data),
        (data) => handleDaemonStderr(data)
    );

    // Step 4: Initialize RPC client
    console.log('[OpenDaemon] Step 4: Initializing RPC client...');
    rpcClient = new RpcClient((data) => {
        if (daemonManager) {
            daemonManager.write(data);
        }
    });

    // Listen for notifications from daemon
    rpcClient.on('notification', (method, params) => {
        handleDaemonNotification(method, params);
    });

    // Step 5: Start daemon
    console.log('[OpenDaemon] Step 5: Starting daemon...');
    try {
        await daemonManager.start(dmnConfigPath);
        console.log('[OpenDaemon] Daemon started successfully');

        // Step 6: Load actual service statuses from daemon
        console.log('[OpenDaemon] Step 6: Loading service statuses from daemon...');
        await loadServices();
        console.log('[OpenDaemon] Initialization complete');
    } catch (err) {
        const errorMessage = err instanceof Error ? err.message : String(err);
        console.error('[OpenDaemon] Failed to start daemon:', errorMessage);

        if (errorDisplayManager) {
            await errorDisplayManager.displayError({
                message: `Failed to start OpenDaemon daemon: ${errorMessage}`,
                category: ErrorCategory.ORCHESTRATOR,
                details: err instanceof Error ? err.stack : undefined
            });
        } else {
            vscode.window.showErrorMessage(
                `Failed to start OpenDaemon: ${errorMessage}`
            );
        }
    }
}
```

## Data Models

### Service Status Flow

```
NotStarted (initial state from config)
    ↓
Starting (daemon spawns process)
    ↓
Running (ready condition met) OR Failed (timeout/error)
    ↓
Stopped (process exited cleanly) OR Failed (process crashed)
```

### Configuration Schema

```typescript
interface DmnConfig {
    version: string;
    services: {
        [serviceName: string]: {
            command: string;
            depends_on?: string[];
            ready_when?: {
                type: 'log_contains' | 'url_responds';
                pattern?: string;  // for log_contains
                url?: string;      // for url_responds
                timeout_seconds?: number;  // optional custom timeout
            };
            env_file?: string;
        };
    };
}
```

## Error Handling

### Error Categories

1. **CONFIG**: Issues parsing or validating dmn.json
2. **ORCHESTRATOR**: Daemon startup or service management failures
3. **RPC**: Communication failures between extension and daemon
4. **SERVICE**: Individual service failures (timeout, crash, etc.)

### Error Display Strategy

```typescript
// For each error category, provide:
// 1. Clear error message
// 2. Actionable next steps
// 3. Links to relevant resources

if (error.category === ErrorCategory.CONFIG) {
    errorDisplayManager.displayError({
        message: error.message,
        category: ErrorCategory.CONFIG,
        actions: [
            { label: 'Open dmn.json', action: () => openDmnConfig() },
            { label: 'View Documentation', action: () => openDocs() }
        ]
    });
}
```

## Testing Strategy

### Unit Tests

1. **Tree View Tests**
   - Test service addition/removal
   - Test status updates
   - Test empty state handling

2. **Config Loading Tests**
   - Test valid config parsing
   - Test invalid config handling
   - Test missing services object

3. **Command Tests**
   - Test command execution with services
   - Test command execution without services
   - Test error handling

### Integration Tests

1. **Extension Activation**
   - Test activation with valid dmn.json
   - Test activation without dmn.json
   - Test activation with invalid dmn.json

2. **Service Lifecycle**
   - Test service discovery
   - Test service startup
   - Test service status updates
   - Test service failure handling

3. **Timeout Scenarios**
   - Test default timeout behavior
   - Test custom timeout configuration
   - Test timeout error messages

### Manual Testing Checklist

- [ ] Extension activates and finds dmn.json
- [ ] Services appear in tree view immediately
- [ ] Services can be started from tree view
- [ ] Services can be started from command palette
- [ ] Service status updates in real-time
- [ ] Timeout errors are clear and actionable
- [ ] Custom timeouts work correctly
- [ ] Error messages provide helpful guidance
