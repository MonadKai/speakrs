use std::path::Path;

use rubato::audioadapter_buffers::owned::InterleavedOwned;
use rubato::{Async, FixedAsync, PolynomialDegree, Resampler};
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::formats::TrackType;
use symphonia::core::formats::probe::Hint;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use thiserror::Error;

pub const SDK_SAMPLE_RATE_HZ: u32 = 16_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioFormat {
    Wav,
    Flac,
    Mp3,
}

impl AudioFormat {
    pub fn from_path(path: &Path) -> Option<Self> {
        let extension = path.extension()?.to_str()?.to_ascii_lowercase();
        match extension.as_str() {
            "wav" | "wave" => Some(Self::Wav),
            "flac" => Some(Self::Flac),
            "mp3" => Some(Self::Mp3),
            _ => None,
        }
    }

    pub fn symphonia_extension(self) -> &'static str {
        match self {
            Self::Wav => "wav",
            Self::Flac => "flac",
            Self::Mp3 => "mp3",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecodedAudio {
    pub samples: Vec<f32>,
    pub sample_rate_hz: u32,
}

impl DecodedAudio {
    pub fn mono_16khz(samples: Vec<f32>) -> Self {
        Self {
            samples,
            sample_rate_hz: SDK_SAMPLE_RATE_HZ,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct InterleavedAudio {
    samples: Vec<f32>,
    sample_rate_hz: u32,
    channels: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResamplePlan {
    pub input_sample_rate_hz: u32,
    pub output_sample_rate_hz: u32,
}

impl ResamplePlan {
    pub fn to_sdk_rate(input_sample_rate_hz: u32) -> Result<Option<Self>, AudioDecodeError> {
        if input_sample_rate_hz == 0 {
            return Err(AudioDecodeError::InvalidSampleRate(input_sample_rate_hz));
        }

        if input_sample_rate_hz == SDK_SAMPLE_RATE_HZ {
            return Ok(None);
        }

        Ok(Some(Self {
            input_sample_rate_hz,
            output_sample_rate_hz: SDK_SAMPLE_RATE_HZ,
        }))
    }

    pub fn ratio(self) -> f64 {
        self.output_sample_rate_hz as f64 / self.input_sample_rate_hz as f64
    }
}

#[derive(Debug, Error)]
pub enum AudioDecodeError {
    #[error("unsupported audio extension for {path}")]
    UnsupportedExtension { path: String },

    #[error("invalid sample rate {0}")]
    InvalidSampleRate(u32),

    #[error("invalid channel count {0}")]
    InvalidChannelCount(usize),

    #[error("audio sample count {samples} is not divisible by channel count {channels}")]
    InvalidInterleavedLength { samples: usize, channels: usize },

    #[error("failed to open audio file `{path}`: {message}")]
    Open { path: String, message: String },

    #[error("failed to probe audio file `{path}`: {message}")]
    Probe { path: String, message: String },

    #[error("audio file `{path}` has no decodable audio track")]
    MissingAudioTrack { path: String },

    #[error("unsupported audio codec in `{path}`: {message}")]
    UnsupportedCodec { path: String, message: String },

    #[error("failed to read audio packet from `{path}`: {message}")]
    ReadPacket { path: String, message: String },

    #[error("failed to decode audio packet from `{path}`: {message}")]
    Decode { path: String, message: String },

    #[error("decoded audio file `{path}` did not contain samples")]
    EmptyAudio { path: String },

    #[error("failed to resample audio: {0}")]
    Resample(String),
}

pub fn supported_formats() -> &'static [AudioFormat] {
    &[AudioFormat::Wav, AudioFormat::Flac, AudioFormat::Mp3]
}

pub fn decode_file_to_mono_16khz(path: impl AsRef<Path>) -> Result<DecodedAudio, AudioDecodeError> {
    let decoded = decode_file_to_mono(path)?;
    let samples = resample_mono_to_16khz(&decoded.samples, decoded.sample_rate_hz)?;
    Ok(DecodedAudio::mono_16khz(samples))
}

pub fn decode_file_to_mono(path: impl AsRef<Path>) -> Result<DecodedAudio, AudioDecodeError> {
    let interleaved = decode_interleaved(path.as_ref())?;
    let samples = downmix_interleaved_to_mono(&interleaved.samples, interleaved.channels)?;
    Ok(DecodedAudio {
        samples,
        sample_rate_hz: interleaved.sample_rate_hz,
    })
}

pub fn downmix_interleaved_to_mono(
    samples: &[f32],
    channels: usize,
) -> Result<Vec<f32>, AudioDecodeError> {
    if channels == 0 {
        return Err(AudioDecodeError::InvalidChannelCount(channels));
    }

    if !samples.len().is_multiple_of(channels) {
        return Err(AudioDecodeError::InvalidInterleavedLength {
            samples: samples.len(),
            channels,
        });
    }

    if channels == 1 {
        return Ok(samples.to_vec());
    }

    let mut mono = Vec::with_capacity(samples.len() / channels);
    for frame in samples.chunks_exact(channels) {
        mono.push(frame.iter().sum::<f32>() / channels as f32);
    }

    Ok(mono)
}

pub fn resample_mono_to_16khz(
    samples: &[f32],
    sample_rate_hz: u32,
) -> Result<Vec<f32>, AudioDecodeError> {
    let Some(plan) = ResamplePlan::to_sdk_rate(sample_rate_hz)? else {
        return Ok(samples.to_vec());
    };

    if samples.is_empty() {
        return Ok(Vec::new());
    }

    let input = InterleavedOwned::new_from(samples.to_vec(), 1, samples.len())
        .map_err(|err| AudioDecodeError::Resample(err.to_string()))?;
    let mut resampler = Async::<f32>::new_poly(
        plan.ratio(),
        1.1,
        PolynomialDegree::Septic,
        1024,
        1,
        FixedAsync::Input,
    )
    .map_err(|err| AudioDecodeError::Resample(err.to_string()))?;
    let output_capacity = resampler.process_all_needed_output_len(samples.len());
    let mut output = InterleavedOwned::new(0.0, 1, output_capacity);
    let (_, output_len) = resampler
        .process_all_into_buffer(&input, &mut output, samples.len(), None)
        .map_err(|err| AudioDecodeError::Resample(err.to_string()))?;

    let mut samples = output.take_data();
    samples.truncate(output_len);
    Ok(samples)
}

fn decode_interleaved(path: &Path) -> Result<InterleavedAudio, AudioDecodeError> {
    let format =
        AudioFormat::from_path(path).ok_or_else(|| AudioDecodeError::UnsupportedExtension {
            path: path.display().to_string(),
        })?;
    let path_display = path.display().to_string();
    let src = std::fs::File::open(path).map_err(|err| AudioDecodeError::Open {
        path: path_display.clone(),
        message: err.to_string(),
    })?;
    let mss = MediaSourceStream::new(Box::new(src), Default::default());
    let mut hint = Hint::new();
    hint.with_extension(format.symphonia_extension());

    let mut format = symphonia::default::get_probe()
        .probe(
            &hint,
            mss,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(|err| AudioDecodeError::Probe {
            path: path_display.clone(),
            message: err.to_string(),
        })?;
    let track = format.default_track(TrackType::Audio).ok_or_else(|| {
        AudioDecodeError::MissingAudioTrack {
            path: path_display.clone(),
        }
    })?;
    let track_id = track.id;
    let codec_params =
        track
            .codec_params
            .as_ref()
            .ok_or_else(|| AudioDecodeError::MissingAudioTrack {
                path: path_display.clone(),
            })?;
    let audio_params = codec_params
        .audio()
        .ok_or_else(|| AudioDecodeError::MissingAudioTrack {
            path: path_display.clone(),
        })?;
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(audio_params, &AudioDecoderOptions::default())
        .map_err(|err| AudioDecodeError::UnsupportedCodec {
            path: path_display.clone(),
            message: err.to_string(),
        })?;

    let mut samples = Vec::new();
    let mut sample_rate_hz = None;
    let mut channels = None;
    while let Some(packet) = next_packet(&mut *format, &path_display)? {
        if packet.track_id != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                let spec = decoded.spec();
                let current_channels = spec.channels().count();
                if current_channels == 0 {
                    return Err(AudioDecodeError::InvalidChannelCount(current_channels));
                }

                sample_rate_hz.get_or_insert(spec.rate());
                channels.get_or_insert(current_channels);

                let mut decoded_samples = vec![0.0; decoded.samples_interleaved()];
                decoded.copy_to_slice_interleaved(&mut decoded_samples);
                samples.extend(decoded_samples);
            }
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(err) => {
                return Err(AudioDecodeError::Decode {
                    path: path_display,
                    message: err.to_string(),
                });
            }
        }
    }

    if samples.is_empty() {
        return Err(AudioDecodeError::EmptyAudio { path: path_display });
    }

    Ok(InterleavedAudio {
        samples,
        sample_rate_hz: sample_rate_hz.unwrap_or(SDK_SAMPLE_RATE_HZ),
        channels: channels.unwrap_or(1),
    })
}

fn next_packet(
    format: &mut dyn symphonia::core::formats::FormatReader,
    path: &str,
) -> Result<Option<symphonia::core::packet::Packet>, AudioDecodeError> {
    match format.next_packet() {
        Ok(packet) => Ok(packet),
        Err(SymphoniaError::ResetRequired) => Err(AudioDecodeError::ReadPacket {
            path: path.to_string(),
            message: "stream reset required".to_string(),
        }),
        Err(SymphoniaError::IoError(err)) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
            Ok(None)
        }
        Err(err) => Err(AudioDecodeError::ReadPacket {
            path: path.to_string(),
            message: err.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::process::Command;

    use rubato::{Async, FixedAsync, Resampler};

    use super::*;

    #[test]
    fn detects_required_file_formats() {
        assert_eq!(
            AudioFormat::from_path(Path::new("input.wav")),
            Some(AudioFormat::Wav)
        );
        assert_eq!(
            AudioFormat::from_path(Path::new("input.FLAC")),
            Some(AudioFormat::Flac)
        );
        assert_eq!(
            AudioFormat::from_path(Path::new("input.mp3")),
            Some(AudioFormat::Mp3)
        );
        assert_eq!(AudioFormat::from_path(Path::new("input.aac")), None);
    }

    #[test]
    fn plans_sdk_resampling_only_when_needed() {
        assert_eq!(ResamplePlan::to_sdk_rate(SDK_SAMPLE_RATE_HZ).unwrap(), None);

        let plan = ResamplePlan::to_sdk_rate(48_000).unwrap().unwrap();
        assert_eq!(plan.input_sample_rate_hz, 48_000);
        assert_eq!(plan.output_sample_rate_hz, SDK_SAMPLE_RATE_HZ);
        assert!((plan.ratio() - (1.0 / 3.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn chosen_resampler_builds_for_sdk_rate_conversion() {
        let plan = ResamplePlan::to_sdk_rate(48_000).unwrap().unwrap();
        let resampler = Async::<f32>::new_poly(
            plan.ratio(),
            1.1,
            rubato::PolynomialDegree::Septic,
            1024,
            1,
            FixedAsync::Input,
        )
        .unwrap();

        assert_eq!(resampler.nbr_channels(), 1);
        assert_eq!(resampler.input_frames_next(), 1024);
        assert!(resampler.output_frames_next() > 0);
    }

    #[test]
    fn downmixes_interleaved_stereo_to_mono() {
        let mono = downmix_interleaved_to_mono(&[0.0, 1.0, -1.0, 1.0], 2).unwrap();

        assert_eq!(mono, vec![0.5, 0.0]);
    }

    #[test]
    fn rejects_invalid_interleaved_audio() {
        let err = downmix_interleaved_to_mono(&[0.0, 1.0, 2.0], 2).unwrap_err();

        assert!(matches!(
            err,
            AudioDecodeError::InvalidInterleavedLength { .. }
        ));
    }

    #[test]
    fn resamples_mono_audio_to_sdk_rate() {
        let input: Vec<f32> = (0..4_800).map(|idx| (idx as f32 / 25.0).sin()).collect();
        let output = resample_mono_to_16khz(&input, 48_000).unwrap();

        assert_eq!(output.len(), 1_600);
    }

    #[test]
    fn decodes_wav_file_to_mono_16khz() {
        let wav = temp_audio_path("decode-stereo-48k", "wav");
        write_stereo_wav(&wav, 48_000, 4_800);

        let decoded = decode_file_to_mono_16khz(&wav).unwrap();

        assert_eq!(decoded.sample_rate_hz, SDK_SAMPLE_RATE_HZ);
        assert_eq!(decoded.samples.len(), 1_600);
        let _ = std::fs::remove_file(wav);
    }

    #[test]
    fn decodes_wav_file_to_mono_before_resampling() {
        let wav = temp_audio_path("decode-mono-48k", "wav");
        write_stereo_wav(&wav, 48_000, 4_800);

        let decoded = decode_file_to_mono(&wav).unwrap();

        assert_eq!(decoded.sample_rate_hz, 48_000);
        assert_eq!(decoded.samples.len(), 4_800);
        let _ = std::fs::remove_file(wav);
    }

    #[test]
    fn decodes_flac_and_mp3_when_encoders_are_available() {
        let wav = temp_audio_path("decode-transcode-source", "wav");
        write_stereo_wav(&wav, 16_000, 1_600);

        for (extension, args) in [
            ("flac", vec!["-y", "-loglevel", "error", "-i"]),
            ("mp3", vec!["-y", "-loglevel", "error", "-i"]),
        ] {
            let encoded = temp_audio_path("decode-transcoded", extension);
            if transcode_with_ffmpeg(&wav, &encoded, &args) {
                let decoded = decode_file_to_mono_16khz(&encoded).unwrap();
                assert_eq!(decoded.sample_rate_hz, SDK_SAMPLE_RATE_HZ);
                assert!(!decoded.samples.is_empty());
                let _ = std::fs::remove_file(encoded);
            }
        }

        let _ = std::fs::remove_file(wav);
    }

    #[test]
    fn rejects_unsupported_audio_extension() {
        let err = decode_file_to_mono_16khz("audio.aac").unwrap_err();

        assert!(matches!(err, AudioDecodeError::UnsupportedExtension { .. }));
    }

    fn write_stereo_wav(path: &Path, sample_rate_hz: u32, frames: usize) {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: sample_rate_hz,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).unwrap();
        for idx in 0..frames {
            let sample = ((idx as f32 / 10.0).sin() * i16::MAX as f32 * 0.5) as i16;
            writer.write_sample(sample).unwrap();
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap();
    }

    fn transcode_with_ffmpeg(source: &Path, destination: &Path, args: &[&str]) -> bool {
        let mut command = Command::new("ffmpeg");
        command.args(args).arg(source).arg(destination);
        command.status().is_ok_and(|status| status.success())
    }

    fn temp_audio_path(name: &str, extension: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "speakrs-sdk-{name}-{}.{extension}",
            std::process::id()
        ))
    }
}
