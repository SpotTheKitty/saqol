// SPDX-License-Identifier: MPL-2.0

// Earwax widens stereo audio for headphone listening. It sends each input
// channel through two 32-tap FIRs that model sound arriving from 30° left and
// 30° right, then mixes the four results into the two headphone channels.
// The coefficients and routing come from FFmpeg's `af_earwax` filter:
// https://github.com/FFmpeg/FFmpeg/blob/master/libavfilter/af_earwax.c

// Copyright (c) 2011 Mina Nagy Zaki
// Copyright (c) 2000 Edward Beingessner And Sundry Contributors.
// This source code is freely redistributable and may be used for any purpose.
// This copyright notice must be maintained. Edward Beingessner And Sundry
// Contributors are not responsible for the consequences of using this software.

const TAP_COUNT: usize = 32;
const COEFFICIENT_SCALE: f32 = 1.0 / 128.0;

// Directional impulse responses for the two virtual loudspeaker positions.
const FILTER_30: [i8; TAP_COUNT] = [
    4, 4, -1, 3, -2, -5, 9, 6, -4, -5, -2, -7, 6, 30, 12, -11, -3, -20, 2, 1, -14, 15, 6, 15, -14,
    -7, -4, 6, 6, 0, 0, 4,
];
const FILTER_330: [i8; TAP_COUNT] = [
    -6, -11, -5, 3, 5, 0, 1, 3, -1, -3, -5, 1, -7, -29, -3, 4, 7, 23, 0, -6, -5, -18, 7, -10, 22,
    -2, 9, -12, -6, -11, -5, 0,
];

pub struct EarwaxEngine {
    // ring buffer
    history: [[f32; TAP_COUNT]; 2],
    oldest: usize,
}

impl EarwaxEngine {
    pub fn new() -> Box<Self> {
        Box::new(Self {
            history: [[0.0; TAP_COUNT]; 2],
            oldest: 0,
        })
    }

    #[inline]
    pub fn process(&mut self, left: f32, right: f32) -> (f32, f32) {
        let left_30 = self.convolve(0, &FILTER_30);
        let left_330 = self.convolve(0, &FILTER_330);
        let right_30 = self.convolve(1, &FILTER_30);
        let right_330 = self.convolve(1, &FILTER_330);

        // `oldest` acts as index to the ring buffer
        self.history[0][self.oldest] = left;
        self.history[1][self.oldest] = right;
        self.oldest = (self.oldest + 1) % TAP_COUNT;

        (left_330 + right_30, left_30 + right_330)
    }

    pub fn reset(&mut self) {
        self.history.fill([0.0; TAP_COUNT]);
        self.oldest = 0;
    }

    #[inline]
    fn convolve(&self, channel: usize, coefficients: &[i8; TAP_COUNT]) -> f32 {
        let mut output = 0.0;
        for (tap, coefficient) in coefficients.iter().enumerate() {
            let sample = self.history[channel][(self.oldest + tap) % TAP_COUNT];
            output += sample * *coefficient as f32;
        }
        output * COEFFICIENT_SCALE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reproduces_ffmpeg_impulse() {
        let mut engine = EarwaxEngine::new();
        assert_eq!(engine.process(1.0, 0.0), (0.0, 0.0));

        for delay in 1..=TAP_COUNT {
            let (left, right) = engine.process(0.0, 0.0);
            let coefficient = TAP_COUNT - delay;
            assert!((left - FILTER_330[coefficient] as f32 / 128.0).abs() < 1e-7);
            assert!((right - FILTER_30[coefficient] as f32 / 128.0).abs() < 1e-7);
        }
        assert_eq!(engine.process(0.0, 0.0), (0.0, 0.0));
    }
}
