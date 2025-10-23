## 🖥 TUI (Terminal UI)

### 19. Main Menu
```
Implement TUI main menu using `ratatui`:
- Share Contact
- Import Contact
- Settings
Use keyboard navigation (↑↓↵) and display current IP.
```

### 20. Share Contact Screen
```
Show generated contact token.
Allow copying to clipboard or saving to file.
Display expiry timestamp.
```

### 21. Import Contact Screen
```
Input field for contact token.
Parse token → ping contact.
Show status: success/failure.
Create active/inactive chat accordingly.
```

### 22. Chat List Screen
```
Display all chats with states:
- Active chats (normal)
- Inactive chats (dimmed)
- Pending messages (highlighted)
Provide actions: Open / Delete.
```

### 23. Chat View
```
Show message history with timestamps.
Provide input box at bottom.
On Enter → send message via core API.
```

### 24. Chat Deletion UX
```
On Delete:
- Confirm via popup.
- If active → send delete request.
- If inactive → delete locally.
Refresh chat list.
```

### 25. Settings Screen
```
Editable retry interval field.
Validate numeric input.
Save automatically on change.
Show confirmation toast.
```

### 26. Status Indicators
```
Implement notification badges:
- New messages → `●`
- Pending delivery → `⌛`
- Expired contact → `⚠`
```

### 27. Queue Sync on Startup
```
On app start, show progress bar for resending pending messages.
Display count of successful/failed retries.
```

---

## 🧪 Tests & Validation

### 28. Contact Token Tests
```
Add tests verifying contact token generation and expiry.
Ensure encoding/decoding symmetry and expiry validation.
```

### 29. Queue Retry Tests
```
Simulate network failures.
Assert exponential backoff and retry limit behavior.
```

### 30. Ping & Message Integration Test
```
Spin up two local HTTP servers (peers).
Exchange pings and messages.
Verify queue flushes successfully.
```

### 31. TUI Smoke Test
```
Add test harness for TUI navigation.
Ensure transitions between Main, Chat, Settings work.
```
