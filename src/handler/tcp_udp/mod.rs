// Copyright 2024 Zinc Labs Inc.
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

use bytes::BytesMut;
use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, UdpSocket},
};

use crate::{job::syslog_server::BROADCASTER, service::logs::syslog};

pub static STOP_SRV: &str = "ZO_STOP_TCP_UDP";

pub async fn udp_server(socket: UdpSocket) {
    let mut buf_udp = vec![0u8; 1472];
    let sender = BROADCASTER.read().await;
    let mut udp_receiver_rx = sender.subscribe();
    loop {
        let (recv_len, addr) = match socket.recv_from(&mut buf_udp).await {
            Ok(val) => val,
            Err(e) => {
                log::error!("Error while reading from UDP socket: {}", e);
                continue;
            }
        };
        let message = BytesMut::from(&buf_udp[..recv_len]);
        let input_str = match String::from_utf8(message.to_vec()) {
            Ok(val) => val,
            Err(e) => {
                log::error!("Error while converting UDP message to UTF8 string: {}", e);
                continue;
            }
        };
        if input_str != STOP_SRV {
            let _ = syslog::ingest(&input_str, addr).await;
        }
        if let Ok(val) = udp_receiver_rx.try_recv() {
            if !val {
                log::warn!("UDP server - received the stop signal, exiting.");
                drop(socket);
                break;
            }
        };
    }
}

pub async fn tcp_server(listener: TcpListener) {
    let sender = BROADCASTER.read().await;
    let mut tcp_receiver_rx = sender.subscribe();
    loop {
        let (mut stream, _) = match listener.accept().await {
            Ok(val) => val,
            Err(e) => {
                log::error!("Error while accepting TCP connection: {}", e);
                continue;
            }
        };
        tokio::task::spawn(async move {
            let mut buf_tcp = vec![0u8; 1460];
            let peer_addr = match stream.peer_addr() {
                Ok(addr) => addr,
                Err(e) => {
                    log::error!("Error while reading peer_addr from TCP stream: {}", e);
                    return;
                }
            };
            log::info!("spawned new syslog tcp receiver for peer {}", peer_addr);
            loop {
                let n = match stream.read(&mut buf_tcp).await {
                    Ok(0) => {
                        log::info!("received 0 bytes, closing for peer {}", peer_addr);
                        break;
                    }
                    Ok(n) => n,
                    Err(e) => {
                        log::error!("Error while reading from TCP stream: {}", e);
                        break;
                    }
                };
                let message = BytesMut::from(&buf_tcp[..n]);
                let input_str = match String::from_utf8(message.to_vec()) {
                    Ok(val) => val,
                    Err(e) => {
                        log::error!("Error while converting TCP message to UTF8 string: {}", e);
                        continue;
                    }
                };
                if input_str != STOP_SRV {
                    let _ = syslog::ingest(&input_str, peer_addr).await;
                }
            }
        });
        if let Ok(val) = tcp_receiver_rx.try_recv() {
            if !val {
                log::warn!("TCP server - received the stop signal, exiting.");
                drop(listener);
                break;
            }
        };
    }
}
