//! Capabilities display functions.

use numaperf::Capabilities;

pub fn print_capabilities(caps: &Capabilities, verbose: bool) {
    println!("Capabilities");
    println!("────────────");

    let status = if caps.supports_hard_mode() {
        "SUPPORTED"
    } else {
        "NOT SUPPORTED"
    };
    println!("Hard mode: {}", status);
    println!();

    let check = |ok: bool| if ok { "[+]" } else { "[-]" };

    println!(
        "  {} CAP_SYS_ADMIN (strict memory binding)",
        check(caps.strict_memory_binding)
    );
    println!(
        "  {} CAP_SYS_NICE (strict CPU affinity)",
        check(caps.strict_cpu_affinity)
    );
    println!(
        "  {} CAP_IPC_LOCK (memory locking)",
        check(caps.memory_locking)
    );
    println!(
        "  {} NUMA balancing disabled",
        check(caps.numa_balancing_disabled)
    );

    if !caps.supports_hard_mode() && verbose {
        println!();
        println!("  To enable hard mode:");
        println!("    - Run as root, or");
        println!("    - sudo setcap cap_sys_admin,cap_sys_nice,cap_ipc_lock+ep <binary>");
        println!("    - echo 0 | sudo tee /proc/sys/kernel/numa_balancing");
    }

    if caps.is_numa_system() {
        println!();
        println!("NUMA system: yes ({} nodes)", caps.numa_node_count);
    } else {
        println!();
        println!("NUMA system: no (single node or UMA)");
    }
}
