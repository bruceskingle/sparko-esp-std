use simple_dns::{Name, CLASS, ResourceRecord, rdata::RData, rdata::A, Packet};
use std::net::{Ipv4Addr, UdpSocket};
use std::thread;
use std::time::Duration;

pub fn start_mdns(hostname: &str, ip_address: &Ipv4Addr) -> anyhow::Result<()> {
    let hostname_local = format!("{}.local", hostname);
    let domain_name = Name::new_unchecked(&hostname_local);

    // Create A record
    let a_record = ResourceRecord::new(
        domain_name.clone(),
        CLASS::IN,
        120,  // TTL
        RData::A(A { address: (*ip_address).into() }),
    );

    // Create DNS response packet
    let mut packet = Packet::new_reply(0); // ID 0 for unsolicited
    packet.answers.push(a_record);

    // Serialize packet
    let packet_data = packet.build_bytes_vec()?;

    // Start background thread to send mDNS announcements
    thread::spawn(move || {
        let socket = match UdpSocket::bind("0.0.0.0:5353") {
            Ok(s) => s,
            Err(e) => {
                log::error!("Failed to bind mDNS socket: {}", e);
                return;
            }
        };

        // Join multicast group
        if let Err(e) = socket.join_multicast_v4(&"224.0.0.251".parse().unwrap(), &"0.0.0.0".parse().unwrap()) {
            log::error!("Failed to join multicast group: {}", e);
            return;
        }

        // Set multicast TTL
        if let Err(e) = socket.set_multicast_ttl_v4(255) {
            log::error!("Failed to set multicast TTL: {}", e);
            return;
        }

        let multicast_addr = "224.0.0.251:5353";

        loop {
            match socket.send_to(&packet_data, multicast_addr) {
                Ok(_) => log::info!("Sent mDNS announcement for {}", hostname_local),
                Err(e) => log::error!("Failed to send mDNS packet: {}", e),
            }
            thread::sleep(Duration::from_secs(60)); // Announce every minute
        }
    });

    log::info!("mDNS advertiser started for: {}.local ({})", hostname, ip_address);
    Ok(())
}