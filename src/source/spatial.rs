use std::time::Duration;

use interpolation;

use crate::source::ChannelVolume;
use crate::{Sample, Source};

use super::SeekError;

/// A simple spatial audio source. The underlying source is transformed to Mono
/// and then played in stereo. The left and right channel's volume are amplified
/// differently depending on the distance of the left and right ear to the source.
#[derive(Clone)]
pub struct Spatial<I>
where
    I: Source,
    I::Item: Sample,
{
    input: ChannelVolume<I>,
}

fn dist_sq(a: [f32; 3], b: [f32; 3]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f32>()
}

impl<I> Spatial<I>
where
    I: Source,
    I::Item: Sample,
{
    /// Builds a new `SpatialSink`, beginning playback on a stream.
    pub fn new(
        input: I,
        emitter_position: [f32; 3],
        left_ear: [f32; 3],
        right_ear: [f32; 3],
    ) -> Spatial<I>
    where
        I: Source,
        I::Item: Sample,
    {
        let mut ret = Spatial {
            input: ChannelVolume::new(input, vec![0.0, 0.0]),
        };
        ret.set_positions(emitter_position, left_ear, right_ear);
        ret
    }

    /// Sets the position of the emitter and ears in the 3D world.
    pub fn set_positions(
        &mut self,
        emitter_pos: [f32; 3],
        left_ear: [f32; 3],
        right_ear: [f32; 3],
    ) {
        debug_assert!(left_ear != right_ear);
        let left_dist_sq = dist_sq(left_ear, emitter_pos);
        let right_dist_sq = dist_sq(right_ear, emitter_pos);
        let max_diff = dist_sq(left_ear, right_ear).sqrt();
        let left_dist = left_dist_sq.sqrt();
        let right_dist = right_dist_sq.sqrt();

        let left_diff_modifier =  (((left_dist - right_dist) / max_diff + 1.0) / 4.0 + 0.5).min(1.0);
        let right_diff_modifier = (((right_dist - left_dist) / max_diff + 1.0) / 4.0 + 0.5).min(1.0);
        // Use a falloff of loudness that is inverse of distance instead of inverse of disttance squared
        let left_dist_modifier = (1.0 / left_dist).min(1.0);
        let right_dist_modifier = (1.0 / right_dist).min(1.0);

        let left_target = left_diff_modifier * left_dist_modifier;
        let right_target = right_diff_modifier * right_dist_modifier;

        let left_vol = self.input.get_volume(0);
        let right_vol = self.input.get_volume(1);

        // lerp to the new target volume.  Lerping helps to smooth out 
        // volume changes to avoid high frequency clicks during position changes.
        let new_left_vol = interpolation::lerp(&left_vol, &left_target, &0.05);
        let new_right_vol = interpolation::lerp(&right_vol, &right_target, &0.05);

        self.input
            .set_volume(0, new_left_vol);
        self.input
            .set_volume(1, new_right_vol);
    }
}

impl<I> Iterator for Spatial<I>
where
    I: Source,
    I::Item: Sample,
{
    type Item = I::Item;

    #[inline]
    fn next(&mut self) -> Option<I::Item> {
        self.input.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.input.size_hint()
    }
}

impl<I> ExactSizeIterator for Spatial<I>
where
    I: Source + ExactSizeIterator,
    I::Item: Sample,
{
}

impl<I> Source for Spatial<I>
where
    I: Source,
    I::Item: Sample,
{
    #[inline]
    fn current_frame_len(&self) -> Option<usize> {
        self.input.current_frame_len()
    }

    #[inline]
    fn channels(&self) -> u16 {
        self.input.channels()
    }

    #[inline]
    fn sample_rate(&self) -> u32 {
        self.input.sample_rate()
    }

    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        self.input.total_duration()
    }

    #[inline]
    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        self.input.try_seek(pos)
    }
}
