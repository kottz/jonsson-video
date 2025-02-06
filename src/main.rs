use macroquad::prelude::*;
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Instant;
use image::{DynamicImage, ImageFormat};

const CURRENT_FORMAT: &str = "avif";
const FRAMES_PER_SHEET: usize = 24;
const FPS: f32 = 15.0;
const FRAME_TIME: f32 = 1.0 / FPS;
const ORIGINAL_WIDTH: f32 = 600.0;
const ORIGINAL_HEIGHT: f32 = 250.0;
const MAX_LOADS_PER_FRAME: usize = 1; // Limit background processing

struct VideoMetadata {
    name: String,
    base_path: String,
    total_frames: usize,
}

struct BackgroundLoader {
    sender: Sender<(usize, DynamicImage, usize)>,
    receiver: Receiver<(usize, DynamicImage, usize)>,
}

impl BackgroundLoader {
    fn new() -> Self {
        let (sender, receiver) = channel();
        Self { sender, receiver }
    }

    fn start_loading(&self, video_index: usize, base_path: String, sheet_index: usize) {
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let path = Path::new(&base_path)
                .join(format!("sprite_sheet_{:03}.{}", sheet_index, CURRENT_FORMAT));
            
            // Read and decode image in background
            if let Ok(data) = std::fs::read(&path) {
                if let Ok(format) = ImageFormat::from_path(&path) {
                    if let Ok(img) = image::load_from_memory_with_format(&data, format) {
                        let _ = sender.send((video_index, img, sheet_index));
                    }
                }
            }
        });
    }
}

struct CutscenePlayer {
    videos: Vec<VideoMetadata>,
    current_video: Option<usize>,
    sprite_sheets: HashMap<usize, VecDeque<Option<Texture2D>>>,
    background_loader: BackgroundLoader,
    audio: Option<macroquad::audio::Sound>,
    playback_start_time: Option<Instant>,
    current_frame: usize,
    is_playing: bool,
    loading: bool,
    loading_progress: f32,
    loading_start_time: Option<Instant>,
    loading_queue: VecDeque<(usize, usize)>,
    show_menu: bool,
}

impl CutscenePlayer {
    async fn new() -> Self {
        let video_names = vec![
            "c_berlin", "c_london", "c_paris", "c_rom", "c_utro 1", "c_utro 2", "intro1", "intro2",
            "iq", "korkeken",
        ];

        let videos = video_names
            .into_iter()
            .map(|name| VideoMetadata {
                name: name.to_string(),
                base_path: format!(
                    "sheet_generator/movies/{}/sprite_sheets/{}",
                    name, CURRENT_FORMAT
                ),
                total_frames: 100 * FRAMES_PER_SHEET,
            })
            .collect();

        Self {
            videos,
            current_video: None,
            sprite_sheets: HashMap::new(),
            background_loader: BackgroundLoader::new(),
            audio: None,
            current_frame: 0,
            playback_start_time: None,
            is_playing: false,
            loading: false,
            loading_progress: 0.0,
            loading_start_time: None,
            loading_queue: VecDeque::new(),
            show_menu: true,
        }
    }

    async fn count_sprite_sheets(&self, base_path: &str) -> usize {
        let mut count = 0;
        loop {
            let path =
                Path::new(base_path).join(format!("sprite_sheet_{:03}.{}", count, CURRENT_FORMAT));
            if !path.exists() {
                break;
            }
            count += 1;
        }
        count
    }

    async fn load_video(&mut self, index: usize) -> bool {
        self.stop();
        self.unload_current_video();
        self.loading = true;
        self.loading_progress = 0.0;
        self.loading_start_time = Some(Instant::now());
        self.show_menu = false;

        let base_path = self.videos[index].base_path.clone();
        let name = self.videos[index].name.clone();

        let total_sheets = self.count_sprite_sheets(&base_path).await;
        self.videos[index].total_frames = total_sheets * FRAMES_PER_SHEET;

        // Initialize video's sprite sheets storage
        self.sprite_sheets.insert(index, VecDeque::new());

        // Load first sheet immediately for playback
        let first_sheet_path =
            Path::new(&base_path).join(format!("sprite_sheet_000.{}", CURRENT_FORMAT));
        if let Ok(texture) = load_texture(first_sheet_path.to_str().unwrap()).await {
            if let Some(sheets) = self.sprite_sheets.get_mut(&index) {
                sheets.push_back(Some(texture));
            }
        }

        // Queue remaining sheets for background loading
        self.loading_queue.clear();
        for sheet_index in 1..total_sheets {
            self.loading_queue.push_back((index, sheet_index));
        }

        // Start loading next sheet in background
        self.start_next_background_load();

        // Load audio
        let audio_path = Path::new("sheet_generator/movies")
            .join(&name)
            .join("audio.wav");
        self.audio = macroquad::audio::load_sound(audio_path.to_str().unwrap())
            .await
            .ok();

        self.current_video = Some(index);
        self.current_frame = 0;
        self.loading = false;

        true
    }

    fn unload_current_video(&mut self) {
        if let Some(video_index) = self.current_video {
            if let Some(audio) = self.audio.take() {
                macroquad::audio::stop_sound(&audio);
            }
            self.sprite_sheets.remove(&video_index);
        }
        self.loading_queue.clear();
    }

    fn start_next_background_load(&mut self) {
        if let Some((video_index, sheet_index)) = self.loading_queue.front() {
            let base_path = self.videos[*video_index].base_path.clone();
            self.background_loader
                .start_loading(*video_index, base_path, *sheet_index);
        }
    }

    fn process_background_loads(&mut self) {
        while let Ok((video_index, img, sheet_index)) = self.background_loader.receiver.try_recv() {
            if let Some(sheets) = self.sprite_sheets.get_mut(&video_index) {
                // Convert decoded image to RGBA
                let rgba = img.to_rgba8();
                let width = img.width();
                let height = img.height();

                // Just create texture from decoded data - this is fast
                let texture = Texture2D::from_rgba8(width as u16, height as u16, &rgba);
                sheets.push_back(Some(texture));

                if let Some(current_video) = self.current_video {
                    if current_video == video_index {
                        let total_sheets = self.loading_queue.len() + sheets.len();
                        self.loading_progress = sheets.len() as f32 / total_sheets as f32;
                    }
                }
            }

            if let Some(front) = self.loading_queue.front() {
                if front.0 == video_index && front.1 == sheet_index {
                    self.loading_queue.pop_front();
                    self.start_next_background_load();
                }
            }
        }
    }

    async fn start_playback(&mut self) {
        if let Some(_) = self.current_video {
            self.current_frame = 0;
            self.playback_start_time = Some(Instant::now());
            self.show_menu = false;

            if let Some(audio) = &self.audio {
                macroquad::audio::play_sound(
                    audio,
                    macroquad::audio::PlaySoundParams {
                        looped: false,
                        volume: 1.0,
                    },
                );
            }

            self.is_playing = true;
        }
    }

    fn stop(&mut self) {
        if let Some(audio) = &self.audio {
            macroquad::audio::stop_sound(audio);
        }
        self.is_playing = false;
        self.current_frame = 0;
        self.playback_start_time = None;
        self.show_menu = true;
    }

    async fn toggle(&mut self, video_index: usize) {
        if self.is_playing && self.current_video == Some(video_index - 1) {
            self.stop();
        } else {
            if self.load_video(video_index - 1).await {
                self.start_playback().await;
            }
        }
    }

    fn draw(&self) {
        clear_background(BLACK);

        if self.show_menu {
            self.draw_menu();
            return;
        }

        if self.is_playing {
            if let Some(video_index) = self.current_video {
                let sheet_index = self.current_frame / FRAMES_PER_SHEET;
                let frame_in_sheet = self.current_frame % FRAMES_PER_SHEET;
                let row = frame_in_sheet / 3;
                let col = frame_in_sheet % 3;

                let src_rect = Rect::new(
                    col as f32 * ORIGINAL_WIDTH,
                    row as f32 * ORIGINAL_HEIGHT,
                    ORIGINAL_WIDTH,
                    ORIGINAL_HEIGHT,
                );

                let (screen_w, screen_h) = (screen_width(), screen_height());
                let scale = (screen_w / ORIGINAL_WIDTH).min(screen_h / ORIGINAL_HEIGHT);
                let scaled_w = ORIGINAL_WIDTH * scale;
                let scaled_h = ORIGINAL_HEIGHT * scale;
                let x = (screen_w - scaled_w) / 2.0;
                let y = (screen_h - scaled_h) / 2.0;

                if let Some(sheets) = self.sprite_sheets.get(&video_index) {
                    if let Some(Some(texture)) = sheets.get(sheet_index) {
                        draw_texture_ex(
                            texture,
                            x,
                            y,
                            WHITE,
                            DrawTextureParams {
                                dest_size: Some(vec2(scaled_w, scaled_h)),
                                source: Some(src_rect),
                                ..Default::default()
                            },
                        );
                    }
                }

                // Draw loading progress if still loading sheets
                if !self.loading_queue.is_empty() {
                    self.draw_loading_progress();
                }
            }
        }
    }

    fn draw_menu(&self) {
        let font_size = 20.0;
        let line_height = font_size * 1.5;
        let start_y = 50.0;

        for (index, video) in self.videos.iter().enumerate() {
            let text = format!("{}: {}", index + 1, video.name);
            let text_dims = measure_text(&text, None, font_size as u16, 1.0);
            let x = (screen_width() - text_dims.width) / 2.0;
            let y = start_y + (index as f32) * line_height;
            draw_text(&text, x, y, font_size, WHITE);
        }

        let instructions = "Press a number key to play/stop a video. Press 'Q' to quit.";
        let instructions_dims = measure_text(instructions, None, font_size as u16, 1.0);
        let instructions_x = (screen_width() - instructions_dims.width) / 2.0;
        let instructions_y = start_y + (self.videos.len() as f32 + 1.0) * line_height;
        draw_text(
            instructions,
            instructions_x,
            instructions_y,
            font_size,
            YELLOW,
        );
    }

    fn draw_loading_progress(&self) {
        let progress_height = 4.0;
        let progress_width = screen_width();
        let y = screen_height() - progress_height;

        // Background
        draw_rectangle(0.0, y, progress_width, progress_height, GRAY);

        // Progress bar
        if let Some(video_index) = self.current_video {
            if let Some(sheets) = self.sprite_sheets.get(&video_index) {
                let total_sheets = self.loading_queue.len() + sheets.len();
                let progress = sheets.len() as f32 / total_sheets as f32;
                draw_rectangle(0.0, y, progress_width * progress, progress_height, GREEN);
            }
        }
    }

    async fn update(&mut self) {
        self.process_background_loads();

        if self.is_playing {
            if let Some(start_time) = self.playback_start_time {
                let elapsed = start_time.elapsed();
                let expected_frame = (elapsed.as_secs_f32() / FRAME_TIME).floor() as usize;

                if let Some(video_index) = self.current_video {
                    let total_frames = self.videos[video_index].total_frames;
                    if expected_frame >= total_frames {
                        self.stop();
                    } else {
                        self.current_frame = expected_frame;
                    }
                }
            }
        }
    }
}

#[macroquad::main("Multi-Video Cutscene Player")]
async fn main() {
    let mut player = CutscenePlayer::new().await;

    loop {
        if !player.loading {
            match get_last_key_pressed() {
                Some(KeyCode::Q) => break,
                Some(key) => match key {
                    KeyCode::Key1 => player.toggle(1).await,
                    KeyCode::Key2 => player.toggle(2).await,
                    KeyCode::Key3 => player.toggle(3).await,
                    KeyCode::Key4 => player.toggle(4).await,
                    KeyCode::Key5 => player.toggle(5).await,
                    KeyCode::Key6 => player.toggle(6).await,
                    KeyCode::Key7 => player.toggle(7).await,
                    KeyCode::Key8 => player.toggle(8).await,
                    KeyCode::Key9 => player.toggle(9).await,
                    KeyCode::Key0 => player.toggle(10).await,
                    _ => (),
                },
                None => (),
            }
        }

        player.update().await;
        player.draw();

        next_frame().await;
    }
    player.stop();
}
