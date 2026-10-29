use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

use crossbeam_channel::Sender;
use windows::core::{GUID, Result};
use windows::Win32::Media::Audio::{
    eConsole, eRender, IAudioCaptureClient, IAudioClient, IMMDeviceEnumerator, MMDeviceEnumerator,
    AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK,
    WAVEFORMATEX, WAVE_FORMAT_PCM, WAVEFORMATEXTENSIBLE,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_MULTITHREADED,
};

use super::PcmF32;

/// KSDATAFORMAT_SUBTYPE_PCM
const KSDATAFORMAT_SUBTYPE_PCM: GUID =
    GUID::from_values(0x0000_0001, 0x0000, 0x0010, [0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71]);
/// KSDATAFORMAT_SUBTYPE_IEEE_FLOAT
const KSDATAFORMAT_SUBTYPE_IEEE_FLOAT: GUID =
    GUID::from_values(0x0000_0003, 0x0000, 0x0010, [0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71]);

const WAVE_FORMAT_EXTENSIBLE: u16 = 0xFFFE;

/// Поток захвата default output в режиме loopback. Шлёт в `tx` блоки `PcmF32`.
pub fn spawn_loopback_capture(tx: Sender<PcmF32>, running: Arc<AtomicBool>) -> JoinHandle<()> {
    std::thread::spawn(move || {
        if let Err(e) = capture_loop(tx, running) {
            eprintln!("loopback: {e:?}");
        }
    })
}

fn capture_loop(tx: Sender<PcmF32>, running: Arc<AtomicBool>) -> Result<()> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
        let audio_client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;

        let mix_format = audio_client.GetMixFormat()?;
        if mix_format.is_null() {
            return Err(windows::core::Error::from(windows::Win32::Foundation::E_POINTER));
        }

        let (channels, bits, rate, is_float) = read_format(mix_format)?;
        let ch = channels.max(1) as usize;

        let mut def_period = 0i64;
        let mut min_period = 0i64;
        let _ = audio_client.GetDevicePeriod(Some(&mut def_period), Some(&mut min_period));
        let buffer_duration = if def_period > 0 {
            def_period
        } else {
            200_000
        };

        audio_client.Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            AUDCLNT_STREAMFLAGS_LOOPBACK,
            buffer_duration,
            0,
            mix_format,
            None,
        )?;

        CoTaskMemFree(Some(mix_format.cast()));

        let capture: IAudioCaptureClient = audio_client.GetService()?;

        audio_client.Start()?;

        while running.load(Ordering::Relaxed) {
            let mut data_ptr: *mut u8 = std::ptr::null_mut();
            let mut frames = 0u32;
            let mut flags = 0u32;
            let mut dev_pos = 0u64;
            let mut qpc = 0u64;

            if capture
                .GetBuffer(
                    &mut data_ptr,
                    &mut frames,
                    &mut flags,
                    Some(&mut dev_pos),
                    Some(&mut qpc),
                )
                .is_err()
            {
                std::thread::sleep(std::time::Duration::from_millis(2));
                continue;
            }

            if frames == 0 {
                let _ = capture.ReleaseBuffer(0);
                std::thread::sleep(std::time::Duration::from_millis(1));
                continue;
            }

            let silent = (flags as i32 & AUDCLNT_BUFFERFLAGS_SILENT.0) != 0;
            let samples = if silent {
                vec![0.0f32; frames as usize * ch]
            } else if is_float {
                decode_f32(data_ptr, frames, ch, bits)?
            } else {
                decode_pcm_int(data_ptr, frames, ch, bits)?
            };

            let _ = capture.ReleaseBuffer(frames);

            if !super::queue::send_or_drop(&tx, PcmF32 {
                sample_rate: rate,
                channels: ch,
                samples,
            }) {
                break;
            }
        }

        let _ = audio_client.Stop();
        Ok(())
    }
}

unsafe fn read_format(wfx: *const WAVEFORMATEX) -> Result<(u16, u16, u32, bool)> {
    let head = &*wfx;
    let tag = head.wFormatTag;

    if tag == WAVE_FORMAT_PCM as u16 {
        return Ok((head.nChannels, head.wBitsPerSample, head.nSamplesPerSec, false));
    }

    if tag == WAVE_FORMAT_EXTENSIBLE {
        let ex = &*(wfx as *const WAVEFORMATEXTENSIBLE);
        let bits = ex.Samples.wValidBitsPerSample;
        let sub = ex.SubFormat;
        let is_float = sub == KSDATAFORMAT_SUBTYPE_IEEE_FLOAT;
        let is_pcm = sub == KSDATAFORMAT_SUBTYPE_PCM;
        if !is_float && !is_pcm {
            return Ok((head.nChannels, head.wBitsPerSample, head.nSamplesPerSec, false));
        }
        return Ok((head.nChannels, bits, head.nSamplesPerSec, is_float));
    }

    Ok((head.nChannels, head.wBitsPerSample, head.nSamplesPerSec, false))
}

unsafe fn decode_f32(
    data_ptr: *mut u8,
    frames: u32,
    channels: usize,
    bits: u16,
) -> Result<Vec<f32>> {
    let n = frames as usize * channels;
    if bits == 32 {
        let sl = std::slice::from_raw_parts(data_ptr as *const f32, n);
        return Ok(sl.to_vec());
    }
    Err(windows::core::Error::from(windows::Win32::Foundation::E_NOTIMPL))
}

unsafe fn decode_pcm_int(
    data_ptr: *mut u8,
    frames: u32,
    channels: usize,
    bits: u16,
) -> Result<Vec<f32>> {
    let n = frames as usize * channels;
    match bits {
        16 => {
            let sl = std::slice::from_raw_parts(data_ptr as *const i16, n);
            Ok(sl.iter().map(|&s| s as f32 / 32768.0).collect())
        }
        24 => {
            let bytes = frames as usize * channels * 3;
            let raw = std::slice::from_raw_parts(data_ptr, bytes);
            let mut out = Vec::with_capacity(n);
            for c in 0..n {
                let i = c * 3;
                let b0 = raw[i] as i32;
                let b1 = raw[i + 1] as i32;
                let b2 = raw[i + 2] as i32;
                let mut v = b0 | (b1 << 8) | (b2 << 16);
                if (v & 0x800000) != 0 {
                    v |= !0xFFFFFF;
                }
                out.push((v as f32) / 8388608.0);
            }
            Ok(out)
        }
        32 => {
            let sl = std::slice::from_raw_parts(data_ptr as *const i32, n);
            Ok(sl.iter().map(|&s| s as f32 / 2147483648.0).collect())
        }
        _ => Err(windows::core::Error::from(windows::Win32::Foundation::E_NOTIMPL)),
    }
}
