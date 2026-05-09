use crate::config::AppConfig;
use std::{
    fs::File,
    io::BufReader,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, OnceLock,
    },
    thread,
    time::{Duration, Instant},
};

static AUDIO_SESSION: OnceLock<Mutex<Option<AudioSession>>> = OnceLock::new();
const TTS_POLL_INTERVAL: Duration = Duration::from_millis(250);
const AUDIO_RETRY_INTERVAL: Duration = Duration::from_secs(1);

struct AudioSession {
    stop: Arc<AtomicBool>,
}

pub fn start_overlay_audio(config: &AppConfig) {
    stop_overlay_audio();

    if !config.overlay_sound_enabled {
        return;
    }

    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = stop.clone();
    let audio_path = config.overlay_sound_path.trim().to_string();
    let voice_text = config.overlay_voice_text.trim().to_string();
    let has_custom_audio = !audio_path.is_empty();

    thread::spawn(move || {
        let mut tts: Option<tts::Tts> = if has_custom_audio {
            None
        } else {
            tts::Tts::default().ok()
        };

        let text = if voice_text.is_empty() {
            "快点学习！"
        } else {
            &voice_text
        };

        let started = Instant::now();
        while !worker_stop.load(Ordering::Relaxed) {
            if has_custom_audio {
                play_wav(&audio_path);
                sleep_until_stopped(&worker_stop, AUDIO_RETRY_INTERVAL);
            } else if let Some(ref mut t) = tts {
                if overlay_audio_should_speak(started.elapsed()) && !tts_is_speaking(t) {
                    let _ = t.speak(text, false);
                }
                sleep_until_stopped(&worker_stop, TTS_POLL_INTERVAL);
            } else {
                sleep_until_stopped(&worker_stop, AUDIO_RETRY_INTERVAL);
            }
        }

        if let Some(ref mut t) = tts {
            let _ = t.stop();
        }
    });

    *session().lock().expect("audio session lock poisoned") = Some(AudioSession { stop });
}

pub fn stop_overlay_audio() {
    if let Some(session) = session()
        .lock()
        .expect("audio session lock poisoned")
        .take()
    {
        session.stop.store(true, Ordering::Relaxed);
    }
}

fn play_wav(path: &str) {
    let Ok(file) = File::open(path) else {
        thread::sleep(Duration::from_secs(1));
        return;
    };

    let buf = BufReader::new(file);
    let Ok(source) = rodio::Decoder::new(buf) else {
        thread::sleep(Duration::from_secs(1));
        return;
    };

    let Ok((_stream, handle)) = rodio::OutputStream::try_default() else {
        return;
    };

    let Ok(sink) = rodio::Sink::try_new(&handle) else {
        return;
    };

    sink.append(source);
    sink.sleep_until_end();
}

fn session() -> &'static Mutex<Option<AudioSession>> {
    AUDIO_SESSION.get_or_init(|| Mutex::new(None))
}

fn overlay_audio_should_speak(_elapsed: Duration) -> bool {
    true
}

fn tts_is_speaking(tts: &tts::Tts) -> bool {
    tts.is_speaking().unwrap_or(false)
}

fn sleep_until_stopped(stop: &AtomicBool, duration: Duration) {
    let started = Instant::now();
    while started.elapsed() < duration && !stop.load(Ordering::Relaxed) {
        thread::sleep(TTS_POLL_INTERVAL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_audio_keeps_speaking_while_overlay_is_active() {
        assert!(overlay_audio_should_speak(Duration::from_secs(0)));
        assert!(overlay_audio_should_speak(Duration::from_secs(30)));
        assert!(overlay_audio_should_speak(Duration::from_secs(180)));
        assert!(overlay_audio_should_speak(Duration::from_secs(600)));
    }
}
