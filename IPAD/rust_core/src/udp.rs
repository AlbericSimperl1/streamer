// // //! UDP-listener voor poort 5000.
// // //!
// // //! Dedicated thread met `std::net::UdpSocket`. Elk pakket gaat direct de ring
// // //! in — geen tussenliggende allocaties. Stats (bytes_total + state) worden via
// // //! een `Arc<Stats>` bijgewerkt.

// // use std::net::UdpSocket;
// // use std::sync::atomic::Ordering;
// // use std::sync::Arc;
// // use std::thread::{self, JoinHandle};

// // use crate::ring::Ring;
// // use crate::stats::{State, Stats};

// // const READ_BUF_SIZE: usize = 65_536; // 64 KiB — ruim boven typische UDP MTU

// // pub struct UdpReceiver {
// //     handle: Option<JoinHandle<()>>,
// // }

// // impl UdpReceiver {
// //     /// Start het luisteren op `port`. Ontvangen pakketten worden in `ring`
// //     /// geschreven; `stats.bytes_total` wordt opgeteld; `on_log` rapporteert
// //     /// statuswijzigingen/fouten.
// //     pub fn start<F>(
// //         port: u16,
// //         ring: Arc<Ring>,
// //         stats: Arc<Stats>,
// //         mut on_log: F,
// //     ) -> std::io::Result<Self>
// //     where
// //         F: FnMut(u8, &str) + Send + 'static,
// //     {
// //         let socket = UdpSocket::bind(("0.0.0.0", port))?;
// //         // 250ms timeout zodat we `running` regelmatig kunnen checken via de
// //         // WouldBlock-tak — ook al is er geen data.
// //         socket.set_read_timeout(Some(std::time::Duration::from_millis(250)))?;
// //         socket.set_broadcast(true)?;

// //         let handle = thread::Builder::new()
// //             .name("hyprpad-udp".into())
// //             .spawn(move || {
// //                 on_log(0, &format!("UDP listening on 0.0.0.0:{}", port));
// //                 stats.set_state(State::Listening);

// //                 let mut buf = [0u8; READ_BUF_SIZE];
// //                 loop {
// //                     match socket.recv_from(&mut buf) {
// //                         Ok((n, _peer)) => {
// //                             if n <= 4 {
// //                                 continue; // header zonder payload, negeer
// //                             }
// //                             let seq = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
// //                             let payload = &buf[4..n];

// //                             // Discontinuity-detectie: sprong > 1 = packet loss of reorder
// //                             let expected = last_seq.wrapping_add(1);
// //                             if initialized && seq != expected {
// //                                 ring.set_discontinuity(true);
// //                                 on_log(1, &format!("seq gap: expected {expected}, got {seq}"));
// //                             }
// //                             last_seq = seq;
// //                             initialized = true;

// //                             ring.write(payload);
// //                             stats.add_bytes((n - 4) as u64);
// //                             if stats.state() == State::Listening as u8 {
// //                                 stats.set_state(State::Receiving);
// //                             }
// //                         }
// //                         Err(ref e)
// //                             if e.kind() == std::io::ErrorKind::WouldBlock
// //                                 || e.kind() == std::io::ErrorKind::TimedOut =>
// //                         {
// //                             continue;
// //                         }
// //                         Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {
// //                             continue;
// //                         }
// //                         Err(ref e) => {
// //                             on_log(2, &format!("UDP fout: {}", e));
// //                             stats.set_state(State::Error);
// //                             break;
// //                         }
// //                     }
// //                 }

// //                 // Socket gesloten — laat de readtimeout lekker verlopen; thread stopt
// //                 // vanzelf als de socket dropt. We zetten hier géén `running` flag meer
// //                 // (drop van UdpReceiver killt de join via stats signaal).
// //                 let _ = stats.state();
// //             })?;

// //         Ok(Self {
// //             handle: Some(handle),
// //         })
// //     }
// // }

// // impl Drop for UdpReceiver {
// //     fn drop(&mut self) {
// //         // Join blokkeurt tot de recv_from-tak terugkeert (binnen 250ms) of de
// //         // socket sluit. We moeten de socket dus niet extern sluiten — drop van
// //         // de UdpSocket in de thread gebeurt automatisch bij thread-exit.
// //         if let Some(h) = self.handle.take() {
// //             let _ = h.join();
// //         }
// //     }
// // }

// // // Stilzwijgend `Ordering` in gebruik voor toekomstige checks in deze module.
// // #[allow(dead_code)]
// // fn _ordering_use() {
// //     let _ = std::sync::atomic::AtomicBool::new(false).load(Ordering::Relaxed);
// // }

// //! UDP-listener voor poort 5000.
// //!
// //! Dedicated thread met `std::net::UdpSocket`. Elk pakket gaat direct de ring
// //! in — geen tussenliggende allocaties. Stats (bytes_total + state) worden via
// //! een `Arc` bijgewerkt.
// use crate::ring::Ring;
// use crate::stats::{State, Stats};
// use std::net::UdpSocket;
// use std::sync::atomic::Ordering;
// use std::sync::Arc;
// use std::thread::{self, JoinHandle};

// const READ_BUF_SIZE: usize = 65_536; // 64 KiB — ruim boven typische UDP MTU

// pub struct UdpReceiver {
//     handle: Option<JoinHandle<()>>,
// }

// impl UdpReceiver {
//     /// Start het luisteren op `port`. Ontvangen pakketten worden in `ring`
//     /// geschreven; `stats.bytes_total` wordt opgeteld; `on_log` rapporteert
//     /// statuswijzigingen/fouten.
//     pub fn start<F>(
//         port: u16,
//         ring: Arc<Ring>,
//         stats: Arc<Stats>,
//         mut on_log: F,
//     ) -> std::io::Result<Self>
//     where
//         F: FnMut(u8, &str) + Send + 'static,
//     {
//         let socket = UdpSocket::bind(("0.0.0.0", port))?;
//         // 250ms timeout zodat we `running` regelmatig kunnen checken via de
//         // WouldBlock-tak — ook al is er geen data.
//         socket.set_read_timeout(Some(std::time::Duration::from_millis(250)))?;
//         socket.set_broadcast(true)?;

//         let handle = thread::Builder::new()
//             .name("hyprpad-udp".into())
//             .spawn(move || {
//                 on_log(0, &format!("UDP listening on 0.0.0.0:{}", port));
//                 stats.set_state(State::Listening);

//                 let mut buf = [0u8; READ_BUF_SIZE];

//                 // === FIX: Declareer deze variabelen hier, boven de loop ===
//                 let mut last_seq: u32 = 0;
//                 let mut initialized = false;

//                 loop {
//                     match socket.recv_from(&mut buf) {
//                         Ok((n, _peer)) => {
//                             if n <= 4 {
//                                 continue; // Header zonder payload, negeer
//                             }

//                             // === FIX: Gebruik wrapping_add (niet warping_add) ===
//                             let seq = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
//                             let payload = &buf[4..n];

//                             // Discontinuity-detectie: sprong > 1 = packet loss of reorder
//                             let expected = last_seq.wrapping_add(1);
//                             if initialized && seq != expected {
//                                 ring.set_discontinuity(true);
//                                 on_log(1, &format!("seq gap: expected {expected}, got {seq}"));
//                             }
//                             last_seq = seq;
//                             initialized = true;

//                             ring.write(payload);
//                             stats.add_bytes((n - 4) as u64);

//                             if stats.state() == State::Listening as u8 {
//                                 stats.set_state(State::Receiving);
//                             }
//                         }
//                         Err(ref e)
//                             if e.kind() == std::io::ErrorKind::WouldBlock
//                                 || e.kind() == std::io::ErrorKind::TimedOut =>
//                         {
//                             continue;
//                         }
//                         Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {
//                             continue;
//                         }
//                         Err(ref e) => {
//                             on_log(2, &format!("UDP fout: {}", e));
//                             stats.set_state(State::Error);
//                             break;
//                         }
//                     }
//                 }
//                 // Socket gesloten — laat de readtimeout lekker verlopen; thread stopt
//                 // vanzelf als de socket dropt. We zetten hier géén `running` flag meer
//                 // (drop van UdpReceiver killt de join via stats signaal).
//                 let _ = stats.state();
//             })?;
//         Ok(Self {
//             handle: Some(handle),
//         })
//     }
// }

// impl Drop for UdpReceiver {
//     fn drop(&mut self) {
//         // Join blokkeurt tot de recv_from-tak terugkeert (binnen 250ms) of de
//         // socket sluit. We moeten de socket dus niet extern sluiten — drop van
//         // de UdpSocket in de thread gebeurt automatisch bij thread-exit.
//         if let Some(h) = self.handle.take() {
//             let _ = h.join();
//         }
//     }
// }

// // Stilzwijgend `Ordering` in gebruik voor toekomstige checks in deze module.
// #[allow(dead_code)]
// fn _ordering_use() {
//     let _ = std::sync::atomic::AtomicBool::new(false).load(Ordering::Relaxed);
// }

//! UDP-listener voor poort 5000.
use crate::ring::Ring;
use crate::stats::{State, Stats};
use std::net::UdpSocket;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread::{self, JoinHandle};

const READ_BUF_SIZE: usize = 65_536;

pub struct UdpReceiver {
    handle: Option<JoinHandle<()>>,
}

impl UdpReceiver {
    pub fn start<F>(
        port: u16,
        ring: Arc<Ring>,
        stats: Arc<Stats>,
        mut on_log: F,
    ) -> std::io::Result<Self>
    where
        F: FnMut(u8, &str) + Send + 'static,
    {
        let socket = UdpSocket::bind(("0.0.0.0", port))?;
        socket.set_read_timeout(Some(std::time::Duration::from_millis(250)))?;
        socket.set_broadcast(true)?;

        let handle = thread::Builder::new()
            .name("hyprpad-udp".into())
            .spawn(move || {
                on_log(0, &format!("UDP listening on 0.0.0.0:{}", port));
                stats.set_state(State::Listening);

                let mut buf = [0u8; READ_BUF_SIZE];
                let mut last_seq: u32 = 0;
                let mut initialized = false;

                loop {
                    match socket.recv_from(&mut buf) {
                        Ok((n, _peer)) => {
                            if n <= 4 {
                                continue; // Header zonder payload, negeer
                            }

                            // Strip de 4-byte seq-header van de PC
                            let seq = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
                            let payload = &buf[4..n];

                            // Discontinuity-detectie
                            let expected = last_seq.wrapping_add(1);
                            if initialized && seq != expected {
                                ring.set_discontinuity(true);
                            }
                            last_seq = seq;
                            initialized = true;

                            ring.write(payload);
                            stats.add_bytes((n - 4) as u64);

                            if stats.state() == State::Listening as u8 {
                                stats.set_state(State::Receiving);
                            }
                        }
                        Err(ref e)
                            if e.kind() == std::io::ErrorKind::WouldBlock
                                || e.kind() == std::io::ErrorKind::TimedOut =>
                        {
                            continue;
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {
                            continue;
                        }
                        Err(ref e) => {
                            on_log(2, &format!("UDP fout: {}", e));
                            stats.set_state(State::Error);
                            break;
                        }
                    }
                }
                let _ = stats.state();
            })?;
        Ok(Self {
            handle: Some(handle),
        })
    }
}

impl Drop for UdpReceiver {
    fn drop(&mut self) {
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

#[allow(dead_code)]
fn _ordering_use() {
    let _ = std::sync::atomic::AtomicBool::new(false).load(Ordering::Relaxed);
}
