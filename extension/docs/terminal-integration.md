# Terminal Integration

OpenDaemon now supports real-time terminal integration for service logs!

## Features

### Named Terminals for Each Service

When you start a service, OpenDaemon creates a dedicated terminal tab with the service name:
- Terminal name format: `dmn: <service-name>`
- Example: `dmn: database`, `dmn: backend-api`, `dmn: frontend`

### Real-Time Log Streaming

Service logs are displayed in real-time in their dedicated terminals, allowing you to:
- See live output as services start and run
- Monitor service health and errors
- Debug issues as they happen

### Interactive Terminals

Unlike log files, these are real VS Code integrated terminals, which means you can:
- Run additional commands in the same terminal
- Interact with the service if needed
- Copy/paste from the terminal
- Search through logs using VS Code's terminal search (Ctrl+F)

## How to Use

### Opening a Service Terminal

There are several ways to open a service's terminal:

1. **From the Tree View:**
   - Right-click on a service in the OpenDaemon Services view
   - Select "Open Terminal"

2. **From the Command Palette:**
   - Press `Ctrl+Shift+P` (or `Cmd+Shift+P` on Mac)
   - Type "OpenDaemon: Open Terminal"
   - Select the service you want to view

3. **Automatic on Service Start:**
   - When you start a service, its terminal is automatically created
   - The terminal will show in the terminal panel with the service name

### Managing Terminals

- **Switch Between Services:** Click on the terminal tabs at the bottom of VS Code
- **Close a Terminal:** Click the trash icon on the terminal tab
- **Clear Terminal:** Use the clear command (`cls` on Windows, `clear` on Unix)
- **Split Terminal:** Right-click on a terminal tab and select "Split Terminal"

## Benefits Over Log Files

### Real-Time Updates
- No need to refresh or reload log files
- See output immediately as it happens
- Better for debugging time-sensitive issues

### Better User Experience
- Familiar terminal interface
- Syntax highlighting and formatting
- Easy to navigate and search
- Can run commands in the same context

### Resource Efficient
- No file I/O overhead
- Logs stay in memory
- Automatic cleanup when terminal is closed

## Terminal Icons

Each service terminal has a distinctive icon:
- 🖥️ Server process icon for easy identification
- Service name clearly displayed in the tab

## Tips

### Viewing Multiple Services
Open terminals for multiple services and use VS Code's split terminal feature to view them side-by-side:
1. Right-click on a terminal tab
2. Select "Split Terminal"
3. Switch to another service's terminal

### Searching Logs
Use VS Code's built-in terminal search:
- Press `Ctrl+F` (or `Cmd+F` on Mac) while focused on a terminal
- Type your search term
- Navigate through matches with Enter

### Keeping Terminals Organized
- Terminals are named with the `dmn:` prefix for easy identification
- Close terminals you're not actively monitoring
- Use the terminal dropdown to quickly switch between services

## Configuration

No additional configuration is needed! Terminal integration works out of the box with your existing `dmn.json` configuration.

## Troubleshooting

### Terminal Not Showing Logs

If a terminal opens but doesn't show logs:
1. Check that the service is actually running (check the tree view status)
2. Verify the service is producing output
3. Try restarting the service

### Terminal Closed Accidentally

If you close a terminal by mistake:
1. Right-click on the service in the tree view
2. Select "Open Terminal" again
3. Recent logs will be fetched and displayed

### Too Many Terminals

If you have many services and terminals become cluttered:
1. Close terminals for services you're not actively debugging
2. Use the terminal dropdown menu to navigate
3. Consider using split terminals to view multiple services at once

## Future Enhancements

Planned improvements for terminal integration:
- Automatic log filtering and highlighting
- Terminal persistence across VS Code restarts
- Custom terminal themes for different service types
- Log level filtering (info, warn, error)
