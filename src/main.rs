use macroquad::prelude::*;
use std::collections::{HashMap, VecDeque};
use std::path::Path;

const FRAMES_PER_SHEET: usize = 24;
const FPS: f32 = 15.0;
const FRAME_TIME: f32 = 1.0 / FPS;
const ORIGINAL_WIDTH: f32 = 600.0;
const ORIGINAL_HEIGHT: f32 = 250.0;
const PRELOAD_SHEETS: usize = 2;

struct Video {
    name: String,
    sprite_sheets: VecDeque<Option<Texture2D>>,
    audio: Option<macroquad::audio::Sound>,
    total_frames: usize,
    base_path: String,
}

struct CutscenePlayer {
    videos: HashMap<usize, Video>,
    current_video: Option<usize>,
    current_frame: usize,
    frame_timer: f32,
    is_playing: bool,
    loading_queue: VecDeque<(usize, usize)>, // (video_index, sheet_index)
}

impl CutscenePlayer {
    async fn new() -> Self {
        let video_names = vec![
            "c_berlin", "c_london", "c_paris", "c_rom", "c_utro 1", "c_utro 2", "intro1", "intro2",
            "iq", "korkeken",
        ];

        let mut videos = HashMap::new();

        for (index, name) in video_names.iter().enumerate() {
            let video = Self::initialize_video(name).await;
            videos.insert(index + 1, video);
        }

        Self {
            videos,
            current_video: None,
            current_frame: 0,
            frame_timer: 0.0,
            is_playing: false,
            loading_queue: VecDeque::new(),
        }
    }

    async fn initialize_video(name: &str) -> Video {
        let base_path = format!("movies/{}/sprite_sheets/png", name);
        let mut sprite_sheets = VecDeque::new();

        // Load only the first sprite sheet
        let first_sheet_path = Path::new(&base_path).join("sprite_sheet_000.png");
        let first_sheet = load_texture(first_sheet_path.to_str().unwrap()).await.ok();
        sprite_sheets.push_back(first_sheet);

        // Initialize the rest as None
        for _ in 1..100 {
            // Assuming a maximum of 100 sprite sheets per video
            sprite_sheets.push_back(None);
        }

        let audio_path = Path::new("movies").join(name).join("audio.wav");
        let audio = macroquad::audio::load_sound(audio_path.to_str().unwrap())
            .await
            .ok();

        let total_frames = sprite_sheets.len() * FRAMES_PER_SHEET;

        Video {
            name: name.to_string(),
            sprite_sheets,
            audio,
            total_frames,
            base_path,
        }
    }

    async fn load_next_texture(&mut self) {
        if let Some((video_index, sheet_index)) = self.loading_queue.pop_front() {
            if let Some(video) = self.videos.get_mut(&video_index) {
                if video.sprite_sheets[sheet_index].is_none() {
                    let path = Path::new(&video.base_path)
                        .join(format!("sprite_sheet_{:03}.png", sheet_index));
                    if let Ok(texture) = load_texture(path.to_str().unwrap()).await {
                        video.sprite_sheets[sheet_index] = Some(texture);
                    }
                }
            }
        }
    }

    fn update(&mut self, dt: f32) {
        if self.is_playing {
            self.frame_timer += dt;
            if self.frame_timer >= FRAME_TIME {
                self.frame_timer -= FRAME_TIME;
                self.current_frame += 1;
                if let Some(video_index) = self.current_video {
                    if let Some(video) = self.videos.get(&video_index) {
                        if self.current_frame >= video.total_frames {
                            self.stop();
                        } else {
                            let current_sheet = self.current_frame / FRAMES_PER_SHEET;
                            let sheets_to_load = current_sheet + PRELOAD_SHEETS;
                            for sheet_index in current_sheet..=sheets_to_load {
                                if sheet_index < video.sprite_sheets.len()
                                    && video.sprite_sheets[sheet_index].is_none()
                                {
                                    self.loading_queue.push_back((video_index, sheet_index));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn draw(&self) {
        clear_background(BLACK);

        if self.is_playing {
            if let Some(video_index) = self.current_video {
                if let Some(video) = self.videos.get(&video_index) {
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

                    if let Some(Some(texture)) = video.sprite_sheets.get(sheet_index) {
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
            }
        } else {
            self.draw_menu();
        }
    }

    fn draw_menu(&self) {
        let font_size = 20.0;
        let line_height = font_size * 1.5;
        let start_y = 50.0;

        for (index, video) in &self.videos {
            let text = format!("{}: {}", index, video.name);
            let text_dims = measure_text(&text, None, font_size as u16, 1.0);
            let x = (screen_width() - text_dims.width) / 2.0;
            let y = start_y + (*index as f32 - 1.0) * line_height;
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

    fn toggle(&mut self, video_index: usize) {
        if self.is_playing && self.current_video == Some(video_index) {
            self.stop();
        } else {
            self.play(video_index);
        }
    }

    fn play(&mut self, video_index: usize) {
        self.stop(); // Stop any currently playing video
        if let Some(video) = self.videos.get(&video_index) {
            self.is_playing = true;
            self.current_video = Some(video_index);
            self.current_frame = 0;
            self.frame_timer = 0.0;

            // Preload the first few sprite sheets
            for i in 0..PRELOAD_SHEETS {
                self.loading_queue.push_back((video_index, i));
            }

            if let Some(audio) = &video.audio {
                macroquad::audio::play_sound(
                    audio,
                    macroquad::audio::PlaySoundParams {
                        looped: false,
                        volume: 1.0,
                    },
                );
            }
        }
    }

    fn stop(&mut self) {
        if let Some(video_index) = self.current_video {
            if let Some(video) = self.videos.get(&video_index) {
                if let Some(audio) = &video.audio {
                    macroquad::audio::stop_sound(audio);
                }
            }
        }
        self.is_playing = false;
        self.current_video = None;
        self.current_frame = 0;
        self.frame_timer = 0.0;
    }
}

#[macroquad::main("Multi-Video Cutscene Player")]
async fn main() {
    let mut player = CutscenePlayer::new().await;

    loop {
        match get_last_key_pressed() {
            Some(KeyCode::Q) => break,
            Some(key) => {
                match key {
                    KeyCode::Key1 => player.toggle(1),
                    KeyCode::Key2 => player.toggle(2),
                    KeyCode::Key3 => player.toggle(3),
                    KeyCode::Key4 => player.toggle(4),
                    KeyCode::Key5 => player.toggle(5),
                    KeyCode::Key6 => player.toggle(6),
                    KeyCode::Key7 => player.toggle(7),
                    KeyCode::Key8 => player.toggle(8),
                    KeyCode::Key9 => player.toggle(9),
                    KeyCode::Key0 => player.toggle(10),
                    _ => (), // Ignore other keys
                }
            }
            None => (), // No key pressed
        }

        player.update(get_frame_time());
        player.draw();
        player.load_next_texture().await;

        next_frame().await
    }
}
