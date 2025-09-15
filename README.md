# GhostForge - Modern Gaming Manager for Linux

<div align="center">

**🎮 Next-Generation Gaming Platform Manager**
*A modern Lutris alternative with Bolt container runtime, real-time monitoring, and advanced gaming optimizations*

![Rust](https://img.shields.io/badge/Rust-2024-orange?logo=rust)
![Bolt](https://img.shields.io/badge/Bolt-Gaming%20Containers-blue?logo=docker)
![ProtonDB](https://img.shields.io/badge/ProtonDB-integrated-green)
![Steam](https://img.shields.io/badge/Steam-compatible-1b2838?logo=steam)
![Battle.net](https://img.shields.io/badge/Battle.net-optimized-0a9ecb?logo=blizzard-entertainment)
![GPU](https://img.shields.io/badge/GPU-Passthrough-green?logo=nvidia)

</div>

---

## 🚀 **What is GhostForge?**

GhostForge is a **next-generation gaming platform manager** designed to completely replace Lutris with modern container technology. Built on the **Bolt gaming container runtime**, it provides isolated gaming environments, GPU passthrough, real-time monitoring, and seamless Wine/Proton integration.

### **Why GhostForge over Lutris?**

- **📦 Bolt Container Runtime**: Isolated gaming environments with GPU passthrough and performance optimization
- **🔄 Real-time Monitoring**: Live container status, resource usage, and game performance tracking
- **🎨 Modern GUI**: Polished egui interface with grid/list views and async state management
- **⚡ Instant Game Launch**: One-click containerized game launching with automatic environment setup
- **🛡️ Safe Isolation**: Games run in isolated containers preventing system conflicts
- **📊 Advanced Metrics**: Real-time CPU, GPU, memory monitoring for optimal gaming performance

---

## Features

* **Proton Variant Manager** – Easily switch and install community or custom Proton builds.
* **Wine Profile Management** – Create isolated configurations for each game.
* **Launcher Integration** – Works with Steam, Blizzard Battle.net, Epic Games Launcher (via Wine), and more.
* **Performance Tweaks** – NVIDIA and AMD optimizations.
* **User-friendly UI & CLI** – Manage games your way.
* **Scriptable Installers** – Share and import game setup scripts.

---

## 🎮 Current Features (v0.1.0)

### **🖥️ Modern GUI Interface**
* **Polished egui Interface** – Beautiful Ocean Blue theme with Material Design principles
* **Multiple View Modes** – Grid view (like Lutris) and detailed list view for game management
* **Real-time Updates** – Live container status and system metrics with auto-refresh
* **Async State Management** – Non-blocking UI with poll-promise for smooth experience

### **📦 Bolt Container Integration**
* **Gaming Containers** – Isolated environments with GPU passthrough and Wine/Proton
* **Container Management** – Start/stop/monitor containers directly from GUI
* **Performance Metrics** – Real-time CPU, memory, GPU usage monitoring
* **Container Details** – Comprehensive information about running gaming environments

### **🎯 Game Management**
* **Library Integration** – Steam and Battle.net game detection and management
* **One-click Launch** – Direct container deployment for games with automatic setup
* **Wine/Proton Profiles** – Dedicated container environments for different Wine versions
* **ProtonDB Integration** – Compatibility ratings and optimization recommendations

### **⚙️ System Integration**
* **Battle.net Optimization** – Specialized setup for Battle.net games with container isolation
* **GPU Passthrough** – NVIDIA/AMD GPU support within gaming containers
* **Network Optimization** – QUIC networking for ultra-low latency gaming
* **Performance Profiling** – Automatic CPU governor and priority optimization

### **🖥️ GUI Usage**

```bash
# Launch the modern GUI (recommended)
forge gui                                      # Start the polished egui interface

# Navigate through:
# • Dashboard - System overview and running containers
# • Games - Library management with grid/list views
# • Containers - Real-time Bolt container monitoring
# • ProtonDB - Game compatibility database
# • Wine/Proton - Version management
# • Graphics - GPU and driver configuration
```

### **⌨️ CLI Commands**

```bash
# Container and game management
forge container launch <game_id>              # Launch game in Bolt container
forge container stop <container_id>           # Stop running container
forge container list                          # List all gaming containers
forge metrics                                 # Show system performance metrics

# Traditional game management
forge game list                               # List all detected games
forge launcher list                           # List configured launchers
forge wine list                               # List Wine/Proton versions

# Battle.net optimizations
forge battlenet setup --game wow              # Setup containerized WoW environment
forge battlenet check                         # Check Bolt compatibility

# System information
forge info --full                             # Full system report with container status
```

## 🚧 Upcoming Features

* **🔍 Advanced Game Discovery** - Automatic Steam/Epic/GOG library scanning
* **🎮 More Launchers** - Epic Games Store, GOG Galaxy, Uplay container support
* **📊 Performance Analytics** - Historical performance tracking and optimization suggestions
* **🌐 Cloud Sync** - Game save synchronization across devices
* **🔧 Custom Containers** - Build your own gaming container images
* **🎯 Mod Management** - Integrated mod installation and management within containers

---

## License

MIT License — Contributions welcome!

