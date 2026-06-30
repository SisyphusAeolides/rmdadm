# rmdadm - Modern RAID Management in Rust

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)]()

A modern, high-performance rewrite of `mdadm` in Rust with a REST API, Web UI dashboard, real-time monitoring, and comprehensive management capabilities for Linux software RAID arrays.

## 🚀 Features

### Core RAID Management
- ✅ **Create, assemble, and manage** RAID arrays (RAID 0, 1, 4, 5, 6, 10)
- ✅ **Device validation** - Comprehensive checks before operations
- ✅ **Metadata support** - Multiple superblock versions (1.0, 1.1, 1.2)
- ✅ **Dry-run mode** - Test operations safely before execution
- ✅ **Array operations** - Add, remove, fail disks dynamically

### Web UI Dashboard
- 🎨 **Modern Interface** - Dark-themed, responsive web dashboard
- 📊 **Real-time Monitoring** - Live array status and health metrics
- 🔐 **Secure Access** - JWT authentication with role-based permissions
- 📱 **Mobile Responsive** - Works on desktop, tablet, and mobile
- ⚡ **Interactive Management** - Create, stop, and manage arrays from the browser

### REST API & Monitoring
- 🔐 **JWT Authentication** - Secure API access with role-based permissions
- 🔑 **API Key Support** - Simple authentication for automation
- 🚦 **Rate Limiting** - Protect against abuse (configurable)
- 📊 **Prometheus Metrics** - Export array health metrics
- 🔔 **Webhook Alerts** - Real-time notifications for degraded arrays
- 📧 **Email Notifications** - SMTP alerts with HTML formatting
- 📚 **OpenAPI/Swagger** - Interactive API documentation
- 💓 **Health Checks** - Service status and uptime monitoring

### Advanced Features
- 🎯 **Background Monitoring** - Automatic array health checks
- 📝 **Structured Logging** - JSON support, file rotation, tracing
- ⚙️ **Configuration File** - YAML-based configuration with env overrides
- 🧪 **Comprehensive Tests** - Unit and integration test coverage
- 🖥️ **Interactive TUI** - Terminal UI for monitoring (planned)
- 📈 **SMART Monitoring** - Disk health tracking

## 📋 Requirements

- **OS**: Linux (kernel 2.6+)
- **Rust**: 1.70 or later
- **Root Access**: Required for RAID operations
- **Dependencies**: 
  - `mdadm` utilities (for some operations)
  - `smartctl` (optional, for SMART monitoring)
  - `blkid`, `blockdev` (for device validation)

## 🔧 Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/SisyphusCode/rmdadm.git
cd rmdadm

# Build release binary
cargo build --release

# Install system-wide
sudo make install

# Install web UI files
sudo mkdir -p /usr/share/rmdadm/web
sudo cp -r web/* /usr/share/rmdadm/web/

# Install systemd service
sudo cp systemd/rmdadm.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable rmdadm.service
sudo systemctl start rmdadm.service
```

### Configuration

```bash
# Create configuration directory
sudo mkdir -p /etc/rmdadm

# Copy example configuration
sudo cp rmdadm.conf.example /etc/rmdadm/rmdadm.conf

# Edit configuration
sudo nano /etc/rmdadm/rmdadm.conf

# Restart service
sudo systemctl restart rmdadm.service
```

## 📖 Usage

### Web Dashboard

Access the modern web interface at `http://localhost:8080/`

**Default Credentials:**
- Username: `admin`
- Password: `changeme` (⚠️ Change in production!)

**Features:**
- Real-time array monitoring with auto-refresh
- System overview with health statistics
- Interactive array management (create, stop, scrub)
- Event logging and notifications
- Mobile-responsive design

### Command Line Interface

#### Create a RAID Array
```bash
# Create RAID1 array with two disks
sudo rmdadm create /dev/md0 --level=1 --raid-devices=2 \
  /dev/sdb1 /dev/sdc1 --metadata=1.2

# Create RAID5 array with custom chunk size
sudo rmdadm create /dev/md1 --level=5 --raid-devices=3 \
  /dev/sdb1 /dev/sdc1 /dev/sdd1 --chunk-size=512
```

#### Assemble an Array
```bash
# Assemble array from components
sudo rmdadm assemble /dev/md0 /dev/sdb1 /dev/sdc1

# Auto-assemble from superblock
sudo rmdadm assemble --auto /dev/sdb1
```

#### Manage Arrays
```bash
# Get array details
sudo rmdadm detail /dev/md0

# Add a spare disk
sudo rmdadm manage /dev/md0 --add /dev/sdd1

# Remove a disk
sudo rmdadm manage /dev/md0 --remove /dev/sdb1

# Mark disk as failed
sudo rmdadm manage /dev/md0 --fail /dev/sdb1

# Stop an array
sudo rmdadm stop /dev/md0
```

#### Monitoring
```bash
# Start interactive monitor
sudo rmdadm monitor

# Start Prometheus exporter
sudo rmdadm exporter

# Start API daemon
sudo rmdadm daemon --addr 0.0.0.0:8080
```

### REST API

#### Authentication

**Login with JWT:**
```bash
# Get JWT token
TOKEN=$(curl -s -X POST http://localhost:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"changeme"}' | jq -r .token)

# Use token in requests
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/arrays
```

**Using API Key:**
```bash
# Set API key in environment
export RMDADM_API_KEY="your-secure-api-key"

# Use API key in requests
curl -H "X-API-Key: your-secure-api-key" \
  http://localhost:8080/api/v1/arrays
```

#### API Endpoints

**Health & Monitoring:**
- `GET /health` - Service health check
- `GET /metrics` - Prometheus metrics
- `GET /api/v1/health` - Detailed health status

**Authentication:**
- `POST /api/v1/auth/login` - Login and get JWT token
- `POST /api/v1/auth/refresh` - Refresh JWT token

**Array Management:**
- `GET /api/v1/arrays` - List all arrays
- `GET /api/v1/arrays/{name}` - Get array details
- `POST /api/v1/arrays` - Create new array (admin only)
- `DELETE /api/v1/arrays/{name}` - Stop array (admin only)
- `POST /api/v1/arrays/{name}/manage` - Manage disks (operator+)
- `POST /api/v1/arrays/{name}/scrub` - Start scrub operation (operator+)

**Interactive Documentation:**
- Visit `http://localhost:8080/swagger-ui/` for full API documentation

### Configuration

#### Configuration File (`/etc/rmdadm/rmdadm.conf`)

```yaml
# Server Configuration
server:
  bind_address: "0.0.0.0:8080"
  enable_tls: false

# Authentication
auth:
  disable_auth: false
  jwt_secret: "your-secret-key-here"
  token_expiry_hours: 24
  admin_user: "admin"
  admin_password: "changeme"

# Rate Limiting
rate_limit:
  enabled: true
  max_requests: 100
  window_seconds: 60

# Monitoring
monitoring:
  enabled: true
  interval_seconds: 60
  webhook_url: "https://hooks.slack.com/services/YOUR/WEBHOOK"
  email:
    smtp_server: "smtp.gmail.com"
    smtp_port: 587
    smtp_username: "your-email@gmail.com"
    smtp_password: "your-app-password"
    from_address: "rmdadm@localhost"
    to_addresses:
      - "admin@example.com"
      - "ops@example.com"

# Logging
logging:
  level: "info"
  log_dir: "/var/log/rmdadm"
  json_format: false
```

#### Environment Variables

Environment variables override configuration file settings:

**Server:**
- `RMDADM_BIND_ADDRESS` - Server bind address

**Authentication:**
- `RMDADM_DISABLE_AUTH` - Disable authentication (dev only)
- `RMDADM_JWT_SECRET` - JWT signing secret
- `RMDADM_API_KEY` - API key for authentication
- `RMDADM_ADMIN_USER` - Admin username
- `RMDADM_ADMIN_PASSWORD` - Admin password

**Rate Limiting:**
- `RMDADM_DISABLE_RATE_LIMIT` - Disable rate limiting
- `RMDADM_RATE_LIMIT_MAX` - Max requests per window
- `RMDADM_RATE_LIMIT_WINDOW` - Time window in seconds

**Monitoring:**
- `RMDADM_WEBHOOK_URL` - Webhook URL for alerts
- `RMDADM_MONITOR_INTERVAL` - Monitoring interval in seconds

**Email Notifications:**
- `RMDADM_EMAIL_ENABLED` - Enable email notifications
- `RMDADM_SMTP_HOST` - SMTP server hostname
- `RMDADM_SMTP_PORT` - SMTP server port
- `RMDADM_SMTP_USERNAME` - SMTP username
- `RMDADM_SMTP_PASSWORD` - SMTP password
- `RMDADM_EMAIL_FROM` - From email address
- `RMDADM_EMAIL_TO` - Comma-separated recipient emails

**Logging:**
- `RUST_LOG` - Log level (trace, debug, info, warn, error)

## 🔐 Security

### Production Deployment Checklist

- [ ] Change default JWT secret (`RMDADM_JWT_SECRET`)
- [ ] Change default admin password (`RMDADM_ADMIN_PASSWORD`)
- [ ] Use strong API keys (32+ characters)
- [ ] Enable TLS/HTTPS in production
- [ ] Configure appropriate rate limits
- [ ] Set up monitoring and alerting
- [ ] Review and restrict API access
- [ ] Enable audit logging
- [ ] Regular security updates
- [ ] Configure email notifications
- [ ] Test backup and recovery procedures

### Role-Based Access Control

- **Admin** - Full access (create, delete, manage arrays)
- **Operator** - Manage existing arrays (add/remove disks, scrub)
- **ReadOnly** - View array status only

## 📊 Monitoring & Alerting

### Prometheus Integration

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'rmdadm'
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: '/metrics'
```

### Webhook Alerts

Configure webhook URL for real-time alerts when arrays become degraded:

```bash
export RMDADM_WEBHOOK_URL="https://hooks.slack.com/services/YOUR/WEBHOOK"
```

Alert format:
```json
{
  "text": "🚨 **rmdadm ALERT**: Array `md0` has entered `degraded` state! Immediate attention required."
}
```

### Email Notifications

Configure SMTP settings for email alerts:

```bash
export RMDADM_EMAIL_ENABLED=true
export RMDADM_SMTP_HOST=smtp.gmail.com
export RMDADM_SMTP_PORT=587
export RMDADM_SMTP_USERNAME=your-email@gmail.com
export RMDADM_SMTP_PASSWORD=your-app-password
export RMDADM_EMAIL_FROM=rmdadm@localhost
export RMDADM_EMAIL_TO=admin@example.com,ops@example.com
```

Email alerts include:
- HTML-formatted messages
- Alert severity levels (Info, Warning, Critical)
- Detailed array information
- Recommended actions
- Direct links to web dashboard

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_health_endpoint

# Run integration tests only
cargo test --test api_tests
```

## 📚 Documentation

- **Web Dashboard**: http://localhost:8080/
- **API Documentation**: http://localhost:8080/swagger-ui/
- **Configuration**: See `rmdadm.conf.example`
- **Man Pages**: `man rmdadm` (after installation)

## 🛠️ Development

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# With all features
cargo build --release --all-features
```

### Code Quality

```bash
# Format code
cargo fmt

# Lint
cargo clippy

# Check
cargo check
```

## 🤝 Contributing

Contributions are welcome! Please follow these guidelines:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines.

### Code Style

- Follow Rust standard formatting (`cargo fmt`)
- Pass all clippy lints (`cargo clippy`)
- Add tests for new features
- Update documentation

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Inspired by the original `mdadm` by Neil Brown
- Built with [Axum](https://github.com/tokio-rs/axum) web framework
- Uses [Tokio](https://tokio.rs/) async runtime
- API documentation with [utoipa](https://github.com/juhaku/utoipa)
- Email notifications with [lettre](https://github.com/lettre/lettre)

## 📞 Support

- **Issues**: [GitHub Issues](https://github.com/SisyphusCode/rmdadm/issues)
- **Discussions**: [GitHub Discussions](https://github.com/SisyphusCode/rmdadm/discussions)
- **Email**: SisyphusCode@protonmail.com

## 🗺️ Roadmap

### Completed ✅
- [x] Web UI dashboard
- [x] Email notifications
- [x] JWT/API key authentication
- [x] Rate limiting
- [x] OpenAPI documentation
- [x] Configuration file support
- [x] Webhook alerts
- [x] Prometheus metrics

### Planned 🚧
- [ ] RAID reshape operations
- [ ] Bitmap support
- [ ] Hot spare management
- [ ] Array migration tools
- [ ] Performance benchmarking
- [ ] Docker container support
- [ ] Kubernetes operator

## ⚠️ Disclaimer

This software is provided "as is" without warranty. Always backup your data before performing RAID operations. Test thoroughly in non-production environments first.

---

**Made with ❤️ and Rust by SisyphusCode**