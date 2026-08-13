/// Map macOS interface names to user-facing labels.
///
/// Heuristic table — good enough for beta without SCNetwork. `en0` is treated as
/// Wi‑Fi on Apple Silicon / recent MacBooks; other `en*` ports are wired.
pub fn friendly_adapter_name(sys_name: &str) -> &'static str {
    let lower = sys_name.to_ascii_lowercase();

    if lower.starts_with("en0") {
        return "Wi-Fi";
    }
    if lower.starts_with("en") {
        return "Ethernet";
    }
    if lower.starts_with("llw") {
        return "Wi-Fi Direct";
    }
    if lower.contains("bridge") {
        return if lower == "bridge0" {
            "Thunderbolt Bridge"
        } else {
            "Bridge"
        };
    }
    if lower.starts_with("utun") {
        return "VPN";
    }
    if lower.starts_with("ipsec") {
        return "VPN";
    }
    if lower.starts_with("awdl") {
        return "AirDrop";
    }
    if lower.starts_with("lo") {
        return "Loopback";
    }
    if lower.starts_with("anpi") {
        return "Accessory";
    }
    if lower.starts_with("ap") {
        return "Personal Hotspot";
    }
    if lower.starts_with("pdp_ip") {
        return "Cellular";
    }
    if lower.starts_with("gif") || lower.starts_with("stf") {
        return "Tunnel";
    }
    if lower.starts_with("xhc") {
        return "USB Ethernet";
    }
    "Network"
}

pub fn adapter_hardware_hint(sys_name: &str) -> &'static str {
    let lower = sys_name.to_ascii_lowercase();

    if lower.starts_with("en0") {
        "802.11 wireless adapter"
    } else if lower.starts_with("en") {
        "Gigabit Ethernet adapter"
    } else if lower.starts_with("llw") {
        "Low-latency Wi-Fi interface"
    } else if lower.starts_with("lo") {
        "Loopback interface"
    } else if lower.starts_with("utun") || lower.starts_with("ipsec") {
        "VPN or iCloud Private Relay tunnel"
    } else if lower.starts_with("awdl") {
        "Apple Wireless Direct Link"
    } else if lower.contains("bridge") {
        "Virtual bridge adapter"
    } else if lower.starts_with("ap") {
        "Personal hotspot interface"
    } else if lower.starts_with("pdp_ip") {
        "Cellular data interface"
    } else if lower.starts_with("xhc") {
        "USB-attached Ethernet"
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn en0_maps_to_wifi() {
        assert_eq!(friendly_adapter_name("en0"), "Wi-Fi");
    }

    #[test]
    fn other_en_ports_map_to_ethernet() {
        assert_eq!(friendly_adapter_name("en5"), "Ethernet");
        assert_eq!(friendly_adapter_name("en1"), "Ethernet");
    }

    #[test]
    fn vpn_and_tunnel_interfaces() {
        assert_eq!(friendly_adapter_name("utun3"), "VPN");
        assert_eq!(friendly_adapter_name("ipsec0"), "VPN");
        assert_eq!(friendly_adapter_name("gif0"), "Tunnel");
    }

    #[test]
    fn bridge0_is_thunderbolt_bridge() {
        assert_eq!(friendly_adapter_name("bridge0"), "Thunderbolt Bridge");
    }

    #[test]
    fn adapter_title_includes_sys_name() {
        assert_eq!(adapter_title("en0"), "Wi-Fi (en0)");
        assert_eq!(adapter_title("utun3"), "VPN (utun3)");
    }
}
