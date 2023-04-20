use firec::{
    config::{network::Interface, Config},
    Machine,
};
use std::{
    fs::File,
    io::copy,
    path::{Path, PathBuf},
};
use tokio::time::{sleep, Duration};

/// This example shows how to create a simple VM with a single vCPU, 1024 MiB of RAM, a root drive and a network interface.
///
/// Requirements:
/// - Firecracker binary at `/usr/bin/firecracker`
/// - Jailer binary at `/usr/bin/jailer`
/// - KVM enabled on your system
///
///
/// It downloads the kernel and rootfs from the Firecracker Quickstart Guide, and use them to boot the VM, be aware that a few
/// hundred MiB of disk space will be used. Once you're done with the example, you can delete the `./examples/simple_vm` directory.
///
/// It uses the jailer feature from Firecracker for enhanced security, you can learn more about it here:
/// https://github.com/firecracker-microvm/firecracker/blob/main/docs/jailer.md

// URLs used are from the Firecracker Quickstart Guide
// ref: https://github.com/firecracker-microvm/firecracker/blob/main/docs/getting-started.md#running-firecracker
fn kernel_url() -> hyper::Uri {
    format!(
        "https://s3.amazonaws.com/spec.ccfc.min/img/quickstart_guide/{}/kernels/vmlinux.bin",
        std::env::consts::ARCH
    )
    .parse::<hyper::Uri>()
    .unwrap()
}

// URLs used are from the Firecracker Quickstart Guide
// ref: https://github.com/firecracker-microvm/firecracker/blob/main/docs/getting-started.md#running-firecracker
fn rootfs_url() -> hyper::Uri {
    format!(
        "https://s3.amazonaws.com/spec.ccfc.min/ci-artifacts/disks/{}/ubuntu-18.04.ext4",
        std::env::consts::ARCH
    )
    .parse::<hyper::Uri>()
    .unwrap()
}

async fn fetch_url(url: hyper::Uri, target_path: PathBuf) {
    if target_path.exists() {
        println!("File already exists, skipping download");
        return;
    }

    let client = reqwest::Client::new();
    let response = client
        .get(url.to_string())
        .send()
        .await
        .expect("Could not download file");
    let mut file = File::create(target_path).expect("Could not create file");

    copy(
        &mut response
            .bytes()
            .await
            .expect("Could not get bytes file into the system")
            .as_ref(),
        &mut file,
    )
    .expect("Could not copy file");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Download the kernel and rootfs in a temporary directory
    std::fs::create_dir_all("./examples/simple_vm").unwrap();
    fetch_url(
        rootfs_url(),
        PathBuf::from("./examples/simple_vm/rootfs.ext4"),
    )
    .await;
    fetch_url(
        kernel_url(),
        PathBuf::from("./examples/simple_vm/kernel.bin"),
    )
    .await;

    // Create a TAP interface between host and guest VM
    // Guest iface name: eth0
    // Host iface name: tap0
    let iface = Interface::new("eth0", "tap0");

    let kernel_args = "console=ttyS0 reboot=k panic=1 pci=off random.trust_cpu=on";
    // Build a config for a microVM with 1 vCPU, 1024 MiB of RAM and a root drive
    let config = Config::builder(None, Path::new("./examples/simple_vm/kernel.bin"))
        .jailer_cfg()
        // Base directory where the jailer will hold its config and files
        .chroot_base_dir(Path::new("./tmp/simple_vm"))
        .exec_file(Path::new("/usr/bin/firecracker"))
        .build()
        .kernel_args(kernel_args)
        .machine_cfg()
        .vcpu_count(1)
        .mem_size_mib(1024)
        .build()
        .add_network_interface(iface)
        // Add drive to the VM configuration by specifying the path to the rootfs
        .add_drive("root", Path::new("./examples/simple_vm/rootfs.ext4"))
        .is_root_device(true)
        .build()
        // Determine where the socket will be handled
        .socket_path(Path::new("./tmp/firec-simple_vm.socket"))
        .build();
    let mut machine = Machine::create(config).await?;

    println!("Booting the VM");
    machine.start().await?;
    println!("Waiting a few seconds, the VM is started at this point");
    sleep(Duration::from_secs(5)).await;
    println!("Shutting down the VM");
    machine.force_shutdown().await?;

    Ok(())
}
