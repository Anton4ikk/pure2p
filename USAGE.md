# Usage Guide

## First Launch

1. **Network Setup**: On first launch, you'll see a yellow warning banner on the main screen indicating network connectivity is being configured. This process establishes your internet-reachable endpoint using IPv6, PCP, NAT-PMP, or UPnP (automatic fallback).

2. **Wait for Connectivity**: Wait until the banner disappears or changes to green. This means:
   - ✓ Your device is exposed to the internet and ready to receive messages
   - Your external IP and port have been detected
   - You can now share your contact information

3. **If Setup Fails**: If all connectivity attempts fail, you'll see a red error banner. Go to **Diagnostics** (press `n` from main menu) to:
   - View detailed connectivity status
   - Check which protocols were attempted
   - Manually retry with `r` or `F5`
   - See CGNAT detection status

## Every Launch

1. **Network Setup**: Automatic connectivity detection (IPv6 → PCP → NAT-PMP → UPnP)
2. **Queue Processing**: After network setup completes, all queued messages/pings are automatically retried
3. **Background Retry**: Continues retrying failed deliveries every N minutes (configurable in Settings)


## Starting Your First Chat

### Option A: You Initiate Contact

1. **Share Your Contact**:
   - From main menu, select **Share Contact** (or press `s`)
   - Your contact token will be displayed (contains your IP, UID, and public keys)
   - Press `c` to copy to clipboard, or `s` to save to file
   - Send this token to the person you want to chat with (via email, messaging app, etc.)

2. **Import Their Contact**:
   - Wait for them to send you their contact token
   - Select **Import Contact** (or press `i` from main menu)
   - Paste their token and press Enter
   - The token will be verified (signature, expiry, format)

3. **Automatic Exchange**:
   - A new chat appears with status **⌛ Pending**
   - Your app automatically sends a ping with your contact token
   - When they're online, they'll receive your ping and be auto-imported into their contacts
   - Once they respond, your chat status changes to **● Active**
   - Both of you are now in each other's contact lists!

### Option B: Someone Imports You First

1. **Receive Ping**: When someone imports your contact token, you'll automatically receive their ping

2. **Auto-Import**: Your app will:
   - Parse and verify their contact token
   - Add them to your contacts automatically
   - Create a new chat with status **● Active**
   - Send a ping response

3. **New Chat Notification**: Check **Chats** to see the new conversation

## Chat Status Indicators

- **⚠ Expired** - Contact token has expired (24-hour default)
- **⌛ Pending** - Waiting for contact to come online and accept your ping
- **● New** - Unread messages available
- **○ Read** - No new messages

## Sending Messages

1. Navigate to **Chats** (press `c` from main menu)
2. Select a chat with arrow keys or `j`/`k`
3. Press Enter to open the chat
4. Type your message and press Enter to send
5. Messages are end-to-end encrypted automatically

## Navigation

- **Main Menu**: `q` or `Esc` to quit, arrow keys to navigate, Enter to select
- **Shortcuts**: `c` = Chats, `s` = Share Contact, `i` = Import Contact, `n` = Diagnostics
- **Back**: Press `Esc` from any screen to go back
- **Delete Chat**: In chat list, press `d` or `Del`, then confirm

## Troubleshooting

### "Pending" Status Won't Clear
- Contact may be offline or unreachable
- Check **Diagnostics** to verify your connectivity is working
- Ask contact to verify their IP hasn't changed
- Try re-importing a fresh contact token

### Messages Not Sending
- Check queue size in **Diagnostics**
- Messages automatically retry with exponential backoff
- Verify contact is online and reachable

### CGNAT Detected
- If you see "⚠️ CGNAT detected" in diagnostics, direct P2P may not work
- You're behind Carrier-Grade NAT (100.64.0.0/10 range)
- Pure2P requires direct connectivity - relay servers are not supported
- Consider using IPv6 if available, or contact your ISP

## Settings

- **Retry Interval**: Change how often failed messages are retried (1-1440 minutes)
- Settings are saved automatically to SQLite database

## Data Storage

All data is stored locally in `./app_data/`:
- `pure2p.db` - Your identity, contacts, chats, messages, settings
- `message_queue.db` - Pending messages retry queue

To reset everything, delete the `app_data` folder while the app is closed.
