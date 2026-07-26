//! H.264 NAL-unit-type constanten. Losgetrokken uit de oude `parser.rs`
//! (Annex-B byte-stream scanner) — die scanner bestaat niet meer. De
//! packetizer aan de PC-kant (`PC/src/capture/packetizer.rs`) levert al
//! precies afgebakende NAL-units per `frame_id`, dus `udp.rs` hoeft alleen
//! nog de chunk-header te lezen en te reassembleren; geen Annex-B-scan meer
//! nodig aan deze kant van de lijn.

pub const NAL_NON_IDR: u8 = 1;
pub const NAL_IDR: u8 = 5;
pub const NAL_SEI: u8 = 6;
pub const NAL_SPS: u8 = 7;
pub const NAL_PPS: u8 = 8;
