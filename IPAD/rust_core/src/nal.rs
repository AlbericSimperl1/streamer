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
/// Access Unit Delimiter — met `-x264-params aud=1` (zie PC-kant) zet x264
/// er precies één van vóór elk frame, ongeacht hoeveel slice-NAL's dat frame
/// heeft. Dit is de betrouwbare, goedkope marker om frame-grenzen te
/// herkennen zonder de slice-header zelf te hoeven parsen.
pub const NAL_AUD: u8 = 9;
