use crate::video::{MediaError, RecordedVideoFrame};
use image::codecs::gif::{GifEncoder, Repeat};
use image::{Delay, Frame, RgbaImage};

pub struct GifExporter;

impl GifExporter {
    /// Export recorded video frames into an animated GIF byte buffer.
    /// `fps`: target playback frames per second (e.g. 15, 24, 30).
    pub fn export_gif(
        frames: &[RecordedVideoFrame],
        fps: u32,
    ) -> Result<Vec<u8>, MediaError> {
        if frames.is_empty() {
            return Err(MediaError::EncodingFailed("No frames to export to GIF".to_string()));
        }

        let fps = fps.clamp(1, 60);
        let delay_num_ms = 1000 / fps;
        let delay = Delay::from_numer_denom_ms(delay_num_ms, 1);

        let mut output = Vec::new();
        {
            let mut encoder = GifEncoder::new(&mut output);
            encoder
                .set_repeat(Repeat::Infinite)
                .map_err(|e| MediaError::EncodingFailed(format!("Failed to set GIF repeat: {:?}", e)))?;

            for f in frames {
                let img = RgbaImage::from_raw(f.width, f.height, f.rgba_data.clone())
                    .ok_or_else(|| MediaError::EncodingFailed("Invalid frame buffer dimensions".to_string()))?;

                let gif_frame = Frame::from_parts(img, 0, 0, delay);
                encoder
                    .encode_frame(gif_frame)
                    .map_err(|e| MediaError::EncodingFailed(format!("Failed to encode GIF frame: {:?}", e)))?;
            }
        }

        Ok(output)
    }
}
