// Copyright 2025 OpenObserve Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

#[cfg(target_os = "linux")]
use std::io::{BufRead, BufReader};

// $ cat /proc/net/tcp
// sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode
// 0: 00000000:13D8 00000000:0000 0A 00000000:00000000 00:00000000 00000000 10000        0 34569220
// 1 000000008bf8b3a1 100 0 0 10 0 1: 00000000:13D9 00000000:0000 0A 00000000:00000000 00:00000000
// 00000000 10000        0 34569119 1 00000000edf72ccf 100 0 0 10 0 2: BB04010A:8C14 2F02010A:13D9
// 01 00000000:00000000 00:00000000 00000000 10000        0 35740169 1 00000000d879165d 21 4 10 17
// -1 3: BB04010A:13D8 2F02010A:9E9E 01 00000000:00000000 00:00000000 00000000 10000        0
// 35652348 1 00000000aa718e3d 21 4 31 10 -1 4: BB04010A:E7D6 7202010A:1538 01 00000000:00000000
// 00:00000000 00000000 10000        0 35885884 1 0000000093584285 21 4 26 10 -1

/// tcp connection state
///
/// 01: ESTABLISHED - The connection is established and data can be exchanged.
/// 02: SYN_SENT - The socket is actively attempting to establish a connection.
/// 03: SYN_RECV - A connection request has been received from the network.
/// 04: FIN_WAIT1 - The socket is closed, and the connection is shutting down.
/// 05: FIN_WAIT2 - The connection is closed, and the socket is waiting for a shutdown.
/// 06: TIME_WAIT - The socket is waiting after close to handle packets still in the network.
/// 07: CLOSE - The socket is not being used.
/// 08: CLOSE_WAIT - The remote end has shut down, waiting for the socket to close.
/// 09: LAST_ACK - The remote end has shut down, and the socket is closed. Waiting for ack.
/// 0A: LISTEN - The socket is listening for incoming connections.
/// 0B: CLOSING - Both sockets are shut down, but not all data has been sent.
#[derive(PartialEq)]
#[allow(dead_code)]
pub enum TcpConnState {
    Established,
    SynSent,
    SynRecv,
    FinWait1,
    FinWait2,
    TimeWait,
    Close,
    CloseWait,
    LastAck,
    Listen,
    Closing,
}

// only support linux for now
#[cfg(target_os = "linux")]
pub fn get_tcp_connections(state: Option<TcpConnState>) -> usize {
    let Ok(file) = std::fs::File::open("/proc/net/tcp") else {
        return 0;
    };
    let reader = BufReader::new(file);
    let Some(state) = state else {
        return reader.lines().count();
    };
    reader
        .lines()
        .filter_map(|line| {
            let Ok(line) = line else {
                return None;
            };
            let parts = line.split_whitespace().collect::<Vec<&str>>();
            if parts.len() < 3 {
                return None;
            }
            let st = parts[3];
            let st = get_tcp_connection_state(st);
            if st.is_some() && st.unwrap() == state {
                return Some(());
            }
            None
        })
        .count()
}

#[cfg(target_os = "linux")]
fn get_tcp_connection_state(state: &str) -> Option<TcpConnState> {
    match state {
        "01" => Some(TcpConnState::Established),
        "02" => Some(TcpConnState::SynSent),
        "03" => Some(TcpConnState::SynRecv),
        "04" => Some(TcpConnState::FinWait1),
        "05" => Some(TcpConnState::FinWait2),
        "06" => Some(TcpConnState::TimeWait),
        "07" => Some(TcpConnState::Close),
        "08" => Some(TcpConnState::CloseWait),
        "09" => Some(TcpConnState::LastAck),
        "0A" => Some(TcpConnState::Listen),
        "0B" => Some(TcpConnState::Closing),
        _ => None,
    }
}

#[cfg(not(target_os = "linux"))]
pub fn get_tcp_connections(_state: Option<TcpConnState>) -> usize {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "linux")]
    fn test_sysinfo_get_tcp_connections() {
        assert!(get_tcp_connections(None) > 0);
        assert!(get_tcp_connections(Some(TcpConnState::Established)) > 0);
    }

    #[test]
    #[cfg(not(target_os = "linux"))]
    fn test_sysinfo_get_tcp_connections() {
        assert_eq!(get_tcp_connections(None), 0);
        assert_eq!(get_tcp_connections(Some(TcpConnState::Established)), 0);
    }
}
