use nicco_core::{Snapshot, Source};

pub fn human(snapshot: &Snapshot) -> String {
    let mut output = format!(
        "nicco status — {}\n",
        match snapshot.source {
            Source::Live => "LIVE (read-only, best-effort snapshot)",
            Source::Demo => "DEMO (synthetic data)",
        }
    );
    if snapshot.interfaces.is_empty() {
        output.push_str("No interfaces observed\n");
    }
    for interface in &snapshot.interfaces {
        output.push_str(&format!(
            "{}: {}  state={}  owner={}\n",
            interface.index,
            interface.name,
            interface.oper_state,
            interface.owner.as_deref().unwrap_or("unknown")
        ));
        for address in &interface.addresses {
            output.push_str(&format!("  {}/{}\n", address.address, address.prefix_len));
        }
        if interface.addresses.is_empty() {
            output.push_str("  No addresses observed\n");
        }
    }
    for backend in &snapshot.backends {
        output.push_str(&format!(
            "{}: runtime-directory={} ({})\n",
            backend.backend, backend.runtime_directory_present, backend.limitation
        ));
    }
    output
}
