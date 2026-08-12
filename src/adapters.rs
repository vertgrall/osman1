/// Map macOS interface names to user-facing labels.
pub fn friendly_adapter_name(sys_name: &str) -> &'static str {
    if sys_name.starts_with("en0") {
        return "Wi-Fi";
    }
    if sys_name.starts_with("en") {
        return "Ethernet";
    }
    if sys_name.contains("bridge") {
        return "Bridge";
    }
    if sys_name.starts_with("utun") {
        return "VPN";
    }
    if sys_name.starts_with("awdl") {
        return "AirDrop";
    }
    if sys_name.starts_with("lo") {
        return "Loopback";
    }
    if sys_name.starts_with("anpi") {
        return "Accessory";
    }
    if sys_name.starts_with("ap") {
        return "Hotspot";
    }
    "Network"
}

pub fn adapter_hardware_hint(sys_name: &str) -> &'static str {
    if sys_name.starts_with("en0") {
        "802.11ax wireless adapter"
    } else if sys_name.starts_with("en") {
        "Gigabit Ethernet adapter"
    } else if sys_name.starts_with("lo") {
        "Loopback interface"
    } else if sys_name.starts_with("utun") {
        "VPN tunnel interface"
    } else if sys_name.starts_with("awdl") {
        "Apple Wireless Direct Link"
    } else if sys_name.contains("bridge") {
        "Virtual bridge adapter"
    } else {
        "Network interface"
    }
}

pub fn adapter_title(sys_name: &str) -> String {
    format!("{} ({})", friendly_adapter_name(sys_name), sys_name)
}

/// Stable canvas key for adapter / lane scoped traces.
pub fn scope_id(context: &str, lane: crate::theme::ProcessLane) -> u64 {
    let mut hash = 0u64;
    for b in context.bytes() {
        hash = hash.wrapping_mul(16777619).wrapping_add(b as u64);
    }
    hash.wrapping_add(match lane {
        crate::theme::ProcessLane::Red => 1,
        crate::theme::ProcessLane::Blue => 2,
        crate::theme::ProcessLane::Green => 3,
    })
}
