//! Standard-speed cassette reconstruction from actual guest MIC transitions.
#[cfg(test)]
use super::CPU_HZ;
use super::{Error, MAX_TAP_BYTES, Result, TapeEdge};
const MAX_EDGES: usize = 1_100_000;
pub struct TapeCapture {
    started: u64,
    cpu_hz: u32,
    edges: Vec<TapeEdge>,
}
fn invalid() -> Error {
    Error::Tape("Recording is incomplete or is not a standard-speed Spectrum cassette")
}
impl TapeCapture {
    #[must_use]
    pub fn new(started: u64) -> Self {
        Self::for_model(started, super::Model::Spectrum48)
    }
    #[must_use]
    pub fn for_model(started: u64, model: super::Model) -> Self {
        Self {
            started,
            cpu_hz: model.cpu_hz(),
            edges: Vec::new(),
        }
    }
    pub fn append(&mut self, edges: &[TapeEdge], now: u64) -> Result<()> {
        if now
            .checked_sub(self.started)
            .is_none_or(|n| n > u64::from(self.cpu_hz) * 600)
            || self
                .edges
                .len()
                .checked_add(edges.len())
                .is_none_or(|n| n > MAX_EDGES)
        {
            return Err(Error::Tape(
                "Cassette recording exceeds ten minutes or its transition limit",
            ));
        }
        let mut previous = self.edges.last().map_or(self.started, |e| e.t_state);
        let mut level = self.edges.last().map(|e| e.high);
        for (i, edge) in edges.iter().enumerate() {
            if edge.t_state < previous
                || edge.t_state > now
                || (edge.t_state == previous && !(self.edges.is_empty() && i == 0))
                || level == Some(edge.high)
            {
                return Err(invalid());
            }
            previous = edge.t_state;
            level = Some(edge.high);
        }
        self.edges.extend_from_slice(edges);
        Ok(())
    }
    pub fn finish(&self) -> Result<Vec<u8>> {
        let pulses: Vec<_> = self
            .edges
            .windows(2)
            .map(|e| e[1].t_state - e[0].t_state)
            .collect();
        let mut cursor = 0;
        let mut tape = Vec::new();
        let mut expected_data = None;
        while cursor < pulses.len() {
            let start = cursor;
            while cursor < pulses.len() && (1850..=2480).contains(&pulses[cursor]) {
                cursor += 1;
            }
            if cursor - start < 256 {
                cursor += 1;
                continue;
            }
            if pulses.get(cursor).is_none_or(|n| !(500..=850).contains(n))
                || pulses
                    .get(cursor + 1)
                    .is_none_or(|n| !(550..=900).contains(n))
            {
                return Err(invalid());
            }
            cursor += 2;
            let mut block = Vec::new();
            let mut byte = 0;
            let mut bits = 0;
            while let (Some(&first), Some(&second)) = (pulses.get(cursor), pulses.get(cursor + 1)) {
                let bit = match (first, second) {
                    (650..=1080, 650..=1080) => 0,
                    (1400..=2020, 1400..=2020) => 1,
                    _ => break,
                };
                byte = (byte << 1) | bit;
                bits += 1;
                cursor += 2;
                if bits == 8 {
                    block.push(byte);
                    byte = 0;
                    bits = 0;
                    if block.len() > 65535 {
                        return Err(invalid());
                    }
                }
            }
            if bits != 0 || block.len() < 2 || block.iter().fold(0, |a, b| a ^ b) != 0 {
                return Err(invalid());
            }
            let len = u16::try_from(block.len()).map_err(|_| invalid())?;
            if let Some(expected) = expected_data.take() {
                if block[0] != 255 || block.len() != expected {
                    return Err(invalid());
                }
            } else if block.len() == 19 && block[0] == 0 && block[1] < 4 {
                expected_data = Some(usize::from(u16::from_le_bytes([block[12], block[13]])) + 2);
            }
            if tape.len() + block.len() + 2 > MAX_TAP_BYTES {
                return Err(invalid());
            }
            tape.extend(len.to_le_bytes());
            tape.extend(block);
        }
        if tape.is_empty() || expected_data.is_some() {
            return Err(invalid());
        }
        Ok(tape)
    }
}
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    #[test]
    fn capture_reconstructs_checked_blocks_and_rejects_partial_or_corrupt_signal() {
        let block = [255, 0, 1, 254];
        let mut edges = vec![TapeEdge {
            t_state: 0,
            high: false,
        }];
        let mut clock = 0;
        let mut high = false;
        for pulse in
            std::iter::repeat_n(2168, 300)
                .chain([667, 735])
                .chain(block.into_iter().flat_map(|byte| {
                    (0..8).flat_map(move |bit| {
                        let n = if byte & (128 >> bit) == 0 { 855 } else { 1710 };
                        [n, n]
                    })
                }))
        {
            clock += pulse;
            high = !high;
            edges.push(TapeEdge {
                t_state: clock,
                high,
            });
        }
        let mut capture = TapeCapture::new(0);
        capture.append(&edges, clock).unwrap();
        assert_eq!(capture.finish().unwrap(), [4, 0, 255, 0, 1, 254]);
        capture.edges.pop();
        assert!(capture.finish().is_err());
        assert!(
            capture
                .append(
                    &[TapeEdge {
                        t_state: 1,
                        high: false
                    }],
                    clock
                )
                .is_err()
        );
        assert!(
            TapeCapture::new(0)
                .append(&[], u64::from(CPU_HZ) * 601)
                .is_err()
        );
        assert!(TapeCapture::new(0).finish().is_err());
    }
}
