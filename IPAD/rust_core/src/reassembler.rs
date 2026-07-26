//! Herbouwt NAL-units uit gechunkte UDP-datagrammen.
//!
//! Elk datagram heeft een 8-byte header:
//!   `[frame_id: u32 BE][chunk_index: u16 BE][total_chunks: u16 BE][payload...]`
//!
//! Deze header wordt aan de zender-kant gezet door
//! `PC/src/capture/packetizer.rs`: elke Annex-B NAL-unit krijgt daar een
//! eigen `frame_id` en wordt in stukken van max. 1300 bytes geknipt. Zodra
//! alle `total_chunks` stukken voor een `frame_id` binnen zijn, plakken we
//! de oorspronkelijke NAL-unit (zonder startcode, mét emulation-prevention-
//! bytes intact) weer aan elkaar.
//!
//! Er is bewust GEEN Annex-B-scanning meer nodig aan deze kant: de
//! packetizer garandeert dat elke `frame_id` exact één NAL-unit
//! vertegenwoordigt, dus we hoeven alleen op frame_id/chunk_index te
//! groeperen — geen last meer van valse startcode-matches in stream-ruis.

use std::collections::HashMap;
use std::time::{Duration, Instant};

pub const HEADER_LEN: usize = 8;

/// Max. aantal gelijktijdig incomplete frames dat we bufferen. Voorkomt
/// ongelimiteerde geheugengroei bij packet loss (een frame dat nooit
/// compleet raakt moet uiteindelijk verdwijnen).
const MAX_PENDING: usize = 32;

/// Frames die langer dan dit bestaan zonder compleet te raken worden
/// opgeruimd — voorkomt dat een verweesde chunk (bv. een SPS waarvan één
/// pakketje verloren ging) voor altijd blijft hangen.
const MAX_AGE: Duration = Duration::from_millis(750);

struct PendingFrame {
    chunks: Vec<Option<Vec<u8>>>,
    received: usize,
    total_len: usize,
    first_seen: Instant,
}

pub struct Reassembler {
    pending: HashMap<u32, PendingFrame>,
    last_sweep: Instant,
}

impl Reassembler {
    pub fn new() -> Self {
        Self {
            pending: HashMap::new(),
            last_sweep: Instant::now(),
        }
    }

    /// Verwerk één binnengekomen UDP-datagram. Retourneert de volledige
    /// NAL-unit-bytes zodra alle chunks van diens `frame_id` binnen zijn.
    ///
    /// Corrupte/te korte pakketjes worden stilzwijgend genegeerd — UDP is
    /// sowieso best-effort, een enkel kapot pakketje mag de stream niet
    /// laten crashen.
    pub fn accept(&mut self, datagram: &[u8]) -> Option<Vec<u8>> {
        if datagram.len() <= HEADER_LEN {
            return None;
        }

        let frame_id = u32::from_be_bytes([datagram[0], datagram[1], datagram[2], datagram[3]]);
        let chunk_index = u16::from_be_bytes([datagram[4], datagram[5]]) as usize;
        let total_chunks = u16::from_be_bytes([datagram[6], datagram[7]]) as usize;
        let payload = &datagram[HEADER_LEN..];

        if total_chunks == 0 || chunk_index >= total_chunks {
            return None; // corrupte/onmogelijke header
        }

        let entry = self
            .pending
            .entry(frame_id)
            .or_insert_with(|| PendingFrame {
                chunks: vec![None; total_chunks],
                received: 0,
                total_len: 0,
                first_seen: Instant::now(),
            });

        // Defensief: als total_chunks niet klopt met een eerder gezien
        // pakketje voor dezelfde frame_id, negeer dit pakketje.
        if entry.chunks.len() != total_chunks {
            return None;
        }

        if entry.chunks[chunk_index].is_none() {
            entry.total_len += payload.len();
            entry.chunks[chunk_index] = Some(payload.to_vec());
            entry.received += 1;
        }

        let result = if entry.received == total_chunks {
            let entry = self.pending.remove(&frame_id).expect("entry present");
            let mut nal = Vec::with_capacity(entry.total_len);
            for c in entry.chunks.into_iter().flatten() {
                nal.extend_from_slice(&c);
            }
            Some(nal)
        } else {
            None
        };

        self.sweep_if_due();
        result
    }

    /// Ruim verweesde frames op: zowel op leeftijd (elke ~250ms gecheckt) als
    /// op aantal (LRU-eviction als MAX_PENDING overschreden wordt).
    fn sweep_if_due(&mut self) {
        if self.last_sweep.elapsed() >= Duration::from_millis(250) {
            self.pending.retain(|_, f| f.first_seen.elapsed() < MAX_AGE);
            self.last_sweep = Instant::now();
        }

        while self.pending.len() > MAX_PENDING {
            let oldest_id = self
                .pending
                .iter()
                .min_by_key(|(_, f)| f.first_seen)
                .map(|(id, _)| *id);
            match oldest_id {
                Some(id) => {
                    self.pending.remove(&id);
                }
                None => break,
            }
        }
    }
}

impl Default for Reassembler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_packet(frame_id: u32, chunk_index: u16, total_chunks: u16, payload: &[u8]) -> Vec<u8> {
        let mut p = Vec::with_capacity(HEADER_LEN + payload.len());
        p.extend_from_slice(&frame_id.to_be_bytes());
        p.extend_from_slice(&chunk_index.to_be_bytes());
        p.extend_from_slice(&total_chunks.to_be_bytes());
        p.extend_from_slice(payload);
        p
    }

    #[test]
    fn single_chunk_frame_reassembles_immediately() {
        let mut r = Reassembler::new();
        let pkt = make_packet(1, 0, 1, b"hello-nal");
        assert_eq!(r.accept(&pkt), Some(b"hello-nal".to_vec()));
    }

    #[test]
    fn multi_chunk_in_order() {
        let mut r = Reassembler::new();
        assert_eq!(r.accept(&make_packet(7, 0, 3, b"AAA")), None);
        assert_eq!(r.accept(&make_packet(7, 1, 3, b"BBB")), None);
        assert_eq!(
            r.accept(&make_packet(7, 2, 3, b"CCC")),
            Some(b"AAABBBCCC".to_vec())
        );
    }

    #[test]
    fn multi_chunk_out_of_order() {
        let mut r = Reassembler::new();
        assert_eq!(r.accept(&make_packet(9, 2, 3, b"CCC")), None);
        assert_eq!(r.accept(&make_packet(9, 0, 3, b"AAA")), None);
        assert_eq!(
            r.accept(&make_packet(9, 1, 3, b"BBB")),
            Some(b"AAABBBCCC".to_vec())
        );
    }

    #[test]
    fn duplicate_chunk_is_ignored_not_double_counted() {
        let mut r = Reassembler::new();
        assert_eq!(r.accept(&make_packet(3, 0, 2, b"AA")), None);
        assert_eq!(r.accept(&make_packet(3, 0, 2, b"AA")), None); // duplicaat
        assert_eq!(
            r.accept(&make_packet(3, 1, 2, b"BB")),
            Some(b"AABB".to_vec())
        );
    }

    #[test]
    fn interleaved_frames_do_not_corrupt_each_other() {
        let mut r = Reassembler::new();
        assert_eq!(r.accept(&make_packet(1, 0, 2, b"F1-A")), None);
        assert_eq!(r.accept(&make_packet(2, 0, 2, b"F2-A")), None);
        assert_eq!(
            r.accept(&make_packet(1, 1, 2, b"F1-B")),
            Some(b"F1-AF1-B".to_vec())
        );
        assert_eq!(
            r.accept(&make_packet(2, 1, 2, b"F2-B")),
            Some(b"F2-AF2-B".to_vec())
        );
    }

    #[test]
    fn corrupt_header_is_ignored() {
        let mut r = Reassembler::new();
        assert_eq!(r.accept(&make_packet(1, 0, 0, b"x")), None); // total_chunks == 0
        assert_eq!(r.accept(&make_packet(1, 5, 3, b"x")), None); // chunk_index >= total_chunks
    }

    #[test]
    fn too_short_datagram_is_ignored() {
        let mut r = Reassembler::new();
        assert_eq!(r.accept(&[0u8; 4]), None);
    }
}
