# firec

[![](https://docs.rs/firec/badge.svg)](https://docs.rs/firec/) [![](https://img.shields.io/crates/v/firec)](https://crates.io/crates/firec)

`firec` (pronounced "fyrek") is Rust client library to interact with [Firecracker]. It allows you to
create, manipulate, query and stop VMMs.

## Examples

```rust,no_run
use std::path::Path;
use firec::{
    Machine,
    config::{Config, Drive, Jailer, Machine as MachineCfg, network::Interface}};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kernel_args = "console=ttyS0 reboot=k panic=1 pci=off random.trust_cpu=on";

    let iface = Interface::new("eth0", "tap0");

    let config = Config::builder("butterfly-dragon-kitty", Path::new("debian-vmlinux"))
        .jailer_cfg()
            .chroot_base_dir(Path::new("/srv"))
            .exec_file(Path::new("/usr/bin/firecracker"))
            .build()
        .kernel_args(kernel_args)
        .machine_cfg()
            .vcpu_count(2)
            .mem_size_mib(1024)
            .build()
        .add_drive("root", Path::new("debian.ext4"))
            .is_root_device(true)
            .build()
        .add_network_interface(iface)
        .socket_path(Path::new("/tmp/firecracker.socket"))
        .build();
    let mut machine = Machine::create(config).await?;

    machine.start().await?;

    // Let the machine run for a bit before we KILL IT :)
    sleep(Duration::from_secs(15)).await;

    machine.force_shutdown().await?;

    Ok(())
}
```

## status

Currently heavily in development and therefore expect a lot of API breakage for a while.

Having said that, we'll be following Cargo's SemVer rules so breaking changes will be released in
new minor releases. However, we will only support the latest release.

[Firecracker]: https://github.com/firecracker-microvm/firecracker/
