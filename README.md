# LitePay Server ⚡

**IoT-first, high-performance Lightning payment server — an LNbits alternative built in Rust.**

LitePay Server is a self-hosted Lightning Network payment platform designed for IoT devices (ESP32, Raspberry Pi) and developers. It provides LNbits-compatible REST APIs with WebSocket real-time notifications, LNURL-pay support, and a path toward a fully decentralized mesh network.

## Why LitePay?

| Feature | LNbits | LitePay |
|---------|--------|---------|
| Language | Python (FastAPI) | **Rust (axum)** |
| Performance | ~500 req/s | **~50,000 req/s** |
| Payment notification | HTTP polling (1.5s delay) | **WebSocket push (<100ms)** |
| IoT support | REST-only | **WebSocket + MQTT (planned)** |
| BOLT12 | No | **Planned** |
| Plugin system | Python-only | **WASM (planned)** |
| Deploy | Python + pip + DB | **Single binary** |

## Quick Start

```bash
# Clone and build
git clone https://github.com/cy4rp/litepay-server.git
cd litepay-server
cargo build --release

# Run with mock Lightning backend (no real node needed)
./target/release/litepay-server

# Server starts at http://0.0.0.0:9000
```

## API (LNbits-compatible)

### Create Wallet
```bash
curl -X POST http://localhost:9000/api/v1/wallets \
  -H "Content-Type: application/json" \
  -d '{"name": "My Wallet"}'
```

### Create Invoice (receive)
```bash
curl -X POST http://localhost:9000/api/v1/payments \
  -H "X-Api-Key: <invoice_key>" \
  -H "Content-Type: application/json" \
  -d '{"amount": 100, "memo": "coffee"}'
```

### Check Payment
```bash
curl http://localhost:9000/api/v1/payments/<checking_id> \
  -H "X-Api-Key: <invoice_key>"
```

### List Payments
```bash
curl http://localhost:9000/api/v1/payments \
  -H "X-Api-Key: <invoice_key>"
```

### LNURL-pay
```bash
# Step 1: Get LNURL-pay metadata
curl http://localhost:9000/lnurlp/<wallet_id>

# Step 2: Get invoice via callback
curl "http://localhost:9000/lnurlp/<wallet_id>/callback?amount=10000"

# QR code (PNG)
curl http://localhost:9000/lnurlp/<wallet_id>/qr -o qr.png
```

### WebSocket (Real-time Notifications)
```javascript
const ws = new WebSocket("ws://localhost:9000/api/v1/ws/<wallet_id>");
ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log("Payment:", data.payment.status, data.payment.amount, "sats");
};
```

## ESP32 Integration

See [`examples/esp32/`](examples/esp32/) for a complete Arduino sketch that:
- Displays LNURL QR code on SSD1306 OLED
- Receives **real-time** payment notifications via WebSocket (no polling!)
- Activates servo motor + LED on payment

**Key improvement over LNbits**: ESP32 connects via WebSocket and receives instant push notifications instead of polling every 1.5 seconds.

## Configuration

Copy `litepay.example.toml` to `litepay.toml`:

```toml
host = "0.0.0.0"
port = 9000
database_url = "sqlite://litepay.db?mode=rwc"

# Mock backend (development)
[ln_backend]
type = "mock"

# LND backend (production)
# [ln_backend]
# type = "lnd"
# url = "https://127.0.0.1:8080"
# macaroon = "0201036c6e640..."
```

## Architecture

```
┌──────────────────────────────────────────────┐
│              LitePay Server (Rust/axum)       │
│                                              │
│  REST API ─── WebSocket ─── LNURL-pay        │
│      │            │             │             │
│  Wallet Engine  Notification  QR Generator   │
│      │          Hub                          │
│  SQLite ──── LN Backend Abstraction          │
│              │          │                    │
│           Mock LND    LND REST               │
└──────────────────────────────────────────────┘
```

## Lightning Backends

| Backend | Status | Use Case |
|---------|--------|----------|
| Mock | ✅ Ready | Development & testing |
| LND (REST) | ✅ Ready | Production |
| CLN | 🔜 Planned | Production |
| Phoenix | 🔜 Planned | Easy setup |
| NWC | 🔜 Planned | Nostr Wallet Connect |

## Roadmap

- [x] **Phase 1 (MVP)**: Wallet, invoices, LNURL-pay, WebSocket, LND backend
- [ ] **Phase 2**: CLN backend, BOLT12, MQTT IoT bridge, WASM plugins
- [ ] **Phase 3**: P2P mesh network, geographic routing, Cashu eCash inter-node settlement

## License

MIT
