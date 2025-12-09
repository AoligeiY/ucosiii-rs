# ucosiii-rs

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-ARM%20Cortex--M4-orange.svg)](https://developer.arm.com/Processors/Cortex-M4)
[![Rust](https://img.shields.io/badge/rust-no__std-red.svg)](https://doc.rust-lang.org/reference/names/preludes.html#the-no_std-attribute)
[![Crates.io](https://img.shields.io/crates/v/ucosiii-rs.svg)](https://crates.io/crates/ucosiii-rs)

> A safe, embedded Rust implementation of the μC/OS-III real-time operating system

A real-time operating system (RTOS) kernel written in Rust, inspired by [μC/OS-III](https://github.com/weston-embedded/uC-OS3). Designed for ARM Cortex-M embedded systems with a focus on **memory safety** and **zero-cost abstractions**.

## ✨ Features

- **Priority-based Preemptive Scheduling** - Up to 64 priority levels with O(1) task selection
- **Synchronization Primitives** - Semaphores and mutexes (with priority inheritance)
- **Time Management** - Tick-based delays with tick wheel optimization
- **Memory Safety** - Leverages Rust's ownership model to prevent common RTOS bugs
- **Zero-cost Abstractions** - No runtime overhead compared to C implementation
- **`defmt` Logging** - Efficient embedded logging via RTT

## 📋 Requirements

> [!NOTE]
> Currently only **ARM Cortex-M** architecture is supported. RISC-V and other architectures may be added in future versions.

### Hardware

| Board                  | MCU           | Status        |
| ---------------------- | ------------- | ------------- |
| STM32F401 Nucleo       | ARM Cortex-M4 | ✅ Tested      |
| Other Cortex-M4 boards | ARM Cortex-M4 | 🔧 Should work |
| Cortex-M3/M0+          | ARM Cortex-M  | ⚠️ Untested    |

- Debug probe: ST-Link (built-in on Nucleo boards) or J-Link

### Software

- [probe-rs](https://probe.rs/) for flashing and debugging
- Target: `thumbv7em-none-eabi` (Cortex-M4/M7) or `thumbv7m-none-eabi` (Cortex-M3)

```bash
# Install ARM target
rustup target add thumbv7em-none-eabi

# Install probe-rs
cargo install probe-rs
```

## 🚀 Quick Start

> [!TIP]
> Default target is **STM32F401**. For other chips, modify `.cargo/config.toml` (runner chip) and `Cargo.toml` (stm32-metapac feature).

### Blink

```bash
cargo run --release --example blink --features pac
```

### Producer-Consumer (Semaphores)

Demonstrates semaphore-based synchronization:

```bash
cargo run --release --example producer_consumer --features pac
```

### Priority Inversion (Mutex)

Shows mutex priority inheritance preventing unbounded priority inversion:

```bash
cargo run --release --example priority_inversion --features pac
```

## 📦 Project Structure

```
ucosiii-rs/
├── src/
│   ├── lib.rs              # Library entry point
│   ├── core/               # Kernel core
│   │   ├── kernel.rs       # os_init, os_start
│   │   ├── task/           # Task management
│   │   ├── sched/          # Priority-based scheduler
│   │   ├── time/           # Time management
│   │   ├── prio.rs         # Priority bitmap operations
│   │   ├── critical.rs     # Critical section handling
│   │   ├── config.rs       # Kernel configuration
│   │   ├── types.rs        # Common type definitions
│   │   └── error.rs        # Error types
│   ├── sync/               # Synchronization primitives
│   │   ├── sem.rs          # Semaphores
│   │   └── mutex.rs        # Mutexes
│   ├── port/               # Hardware abstraction layer
│   │   └── arm_cm4/        # ARM Cortex-M4 port
│   └── log.rs              # defmt logging macros
├── examples/
│   ├── blink.rs            # Single task LED blink
│   ├── producer_consumer.rs # Semaphore synchronization demo
│   └── priority_inversion.rs # Mutex priority inheritance demo
├── Cargo.toml
└── README.md
```

## ⚙️ Configuration

### Cargo Features

| Feature    | Default | Description                           |
| ---------- | ------- | ------------------------------------- |
| `full`     | ✅       | Enable all available sync primitives  |
| `sem`      | ✅       | Semaphore support                     |
| `mutex`    | ✅       | Mutex with priority inheritance       |
| `defmt`    | ✅       | Enable defmt logging via RTT          |
| `pac`      | ✅       | Include STM32 peripheral access crate |
| `hal`      | ❌       | Include STM32F4xx HAL                 |
| `memory-x` | ✅       | Use memory.x from stm32-metapac       |
| `rt`       | ❌       | Runtime support from stm32-metapac    |

## 🤝 Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.

## 📄 License

This project is licensed under the Apache-2.0 License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [Micriμm μC/OS-III](https://github.com/weston-embedded/uC-OS3) - Original C implementation
- [Embassy](https://embassy.dev/) - Inspiration for Rust embedded async patterns
- [cortex-m](https://github.com/rust-embedded/cortex-m) - Low-level ARM Cortex-M support