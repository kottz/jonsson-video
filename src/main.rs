use macroquad::prelude::*;
use std::collections::VecDeque;
use std::path::Path;

const FRAMES_PER_SHEET: usize = 24;
const FPS: f32 = 15.0;
const FRAME_TIME: f32 = 1.0 / FPS;
const ORIGINAL_WIDTH: f32 = 600.0;
const ORIGINAL_HEIGHT: f32 = 250.0;
const PRELOAD_SHEETS: usize = 4;

struct VideoMetadata {
    name: String,
    base_path: String,
    total_frames: usize,
}

struct CutscenePlayer {
    videos: Vec<VideoMetadata>,
    current_video: Option<usize>,
    sprite_sheets: VecDeque<Option<Texture2D>>,
    audio: Option<macroquad::audio::Sound>,
    current_frame: usize,
    frame_timer: f32,
    is_playing: bool,
    loading_queue: VecDeque<usize>, // sheet_index
    loading: bool,
    loading_progress: f32,
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
                base_path: format!("movies/{}/sprite_sheets/png", name),
                total_frames: 100 * FRAMES_PER_SHEET, // Assume 100 sheets max, update this if needed
            })
            .collect();

        Self {
            videos,
            current_video: None,
            sprite_sheets: VecDeque::new(),
            audio: None,
            current_frame: 0,
            frame_timer: 0.0,
            is_playing: false,
            loading_queue: VecDeque::new(),
            loading: false,
            loading_progress: 0.0,
        }
    }

    async fn load_video(&mut self, index: usize) -> bool {
        self.stop();
        self.unload_current_video();
        self.loading = true;
        self.loading_progress = 0.0;

        let base_path = self.videos[index].base_path.clone();
        let name = self.videos[index].name.clone();

        // Clear the screen once before starting the loading process
        clear_background(BLACK);
        self.draw_loading_screen();
        next_frame().await;

        // Load all sprite sheets
        self.sprite_sheets.clear();
        let mut sheet_index = 0;
        let total_sheets = 100; // Assume 100 sheets max, adjust if needed
        loop {
            let path = Path::new(&base_path).join(format!("sprite_sheet_{:03}.png", sheet_index));
            match load_texture(path.to_str().unwrap()).await {
                Ok(texture) => {
                    self.sprite_sheets.push_back(Some(texture));
                    sheet_index += 1;
                    self.loading_progress = sheet_index as f32 / total_sheets as f32;

                    // Update loading screen without clearing the background
                    self.draw_loading_screen();
                    next_frame().await;
                }
                Err(_) => break,
            }
        }

        // Update total frames based on actual loaded sheets
        self.videos[index].total_frames = self.sprite_sheets.len() * FRAMES_PER_SHEET;

        let audio_path = Path::new("movies").join(&name).join("audio.wav");
        self.audio = macroquad::audio::load_sound(audio_path.to_str().unwrap())
            .await
            .ok();

        self.current_video = Some(index);
        self.current_frame = 0;
        self.frame_timer = 0.0;
        self.is_playing = false;
        self.loading = false;
        self.loading_progress = 1.0;

        // Clear the screen one last time after loading is complete
        clear_background(BLACK);
        next_frame().await;

        true // Return true to indicate successful loading
    }

    async fn start_playback(&mut self) {
        if let Some(_) = self.current_video {
            // Reset timing variables
            self.current_frame = 0;
            self.frame_timer = 0.0;

            // Small delay to ensure everything is ready
            let start_time = get_time();
            while get_time() - start_time < 0.1 {
                next_frame().await;
            }

            // Start audio playback
            if let Some(audio) = &self.audio {
                macroquad::audio::play_sound(
                    audio,
                    macroquad::audio::PlaySoundParams {
                        looped: false,
                        volume: 1.0,
                    },
                );
            }

            // Start video playback
            self.is_playing = true;
        }
    }
    async fn load_next_texture(&mut self) {
        if let Some(sheet_index) = self.loading_queue.pop_front() {
            if let Some(video_index) = self.current_video {
                let metadata = &self.videos[video_index];
                let path = Path::new(&metadata.base_path)
                    .join(format!("sprite_sheet_{:03}.png", sheet_index));
                if let Ok(texture) = load_texture(path.to_str().unwrap()).await {
                    if sheet_index < self.sprite_sheets.len() {
                        self.sprite_sheets[sheet_index] = Some(texture);
                    }
                }
            }
        }
    }

    fn unload_current_video(&mut self) {
        self.sprite_sheets.clear();
        if let Some(audio) = self.audio.take() {
            macroquad::audio::stop_sound(&audio);
        }
    }

    fn draw(&self) {
        if !self.loading {
            clear_background(BLACK);
        }

        if self.loading {
            self.draw_loading_screen();
        } else if self.is_playing {
            if let Some(_) = self.current_video {
                // Changed to use underscore
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

                if let Some(Some(texture)) = self.sprite_sheets.get(sheet_index) {
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
        } else {
            self.draw_menu();
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

    fn draw_loading_screen(&self) {
        let screen_width = screen_width();
        let screen_height = screen_height();
        let bar_width = screen_width * 0.8;
        let bar_height = 20.0;
        let bar_x = (screen_width - bar_width) / 2.0;
        let bar_y = screen_height / 2.0;

        // Draw background bar
        draw_rectangle(bar_x, bar_y, bar_width, bar_height, GRAY);

        // Draw progress bar
        let progress_width = bar_width * self.loading_progress;
        draw_rectangle(bar_x, bar_y, progress_width, bar_height, GREEN);

        // Draw text
        let text = "Loading...";
        let font_size = 30.0;
        let text_dims = measure_text(text, None, font_size as u16, 1.0);
        draw_text(
            text,
            (screen_width - text_dims.width) / 2.0,
            bar_y - 40.0,
            font_size,
            WHITE,
        );
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

    fn update(&mut self, dt: f32) {
        if self.is_playing {
            self.frame_timer += dt;
            while self.frame_timer >= FRAME_TIME {
                self.frame_timer -= FRAME_TIME;
                self.current_frame += 1;
                if let Some(video_index) = self.current_video {
                    let total_frames = self.videos[video_index].total_frames;
                    if self.current_frame >= total_frames {
                        self.stop();
                        break;
                    }
                }
            }
        }
    }

    fn stop(&mut self) {
        if let Some(audio) = &self.audio {
            macroquad::audio::stop_sound(audio);
        }
        self.is_playing = false;
        self.current_frame = 0;
        self.frame_timer = 0.0;
    }
}

#[macroquad::main("Multi-Video Cutscene Player")]
async fn main() {
    let mut player = CutscenePlayer::new().await;

    loop {
        if !player.loading {
            match get_last_key_pressed() {
                Some(KeyCode::Q) => break,
                Some(key) => {
                    match key {
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
                        _ => (), // Ignore other keys
                    }
                }
                None => (), // No key pressed
            }
        }

        player.update(get_frame_time());
        player.draw();

        next_frame().await;
    }

    player.stop();
}
