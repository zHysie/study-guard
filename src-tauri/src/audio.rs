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
    let burst = Duration::from_secs(config.overlay_sound_burst_seconds.max(1) as u64);
    let pause = Duration::from_secs(config.overlay_sound_pause_minutes.max(1) as u64 * 60);

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

        while !worker_stop.load(Ordering::Relaxed) {
            let burst_started = Instant::now();
            while burst_started.elapsed() < burst && !worker_stop.load(Ordering::Relaxed) {
                if has_custom_audio {
                    play_wav(&audio_path);
                } else if let Some(ref mut t) = tts {
                    let _ = t.speak(text, true);
                }
            }

            let pause_started = Instant::now();
            while pause_started.elapsed() < pause && !worker_stop.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(250));
            }
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
