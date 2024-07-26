use macroquad::prelude::*;

const SPRITE_SHEET_COUNT: usize = 29;
const FRAMES_PER_SHEET: usize = 24;
const FPS: f32 = 15.0;
const FRAME_TIME: f32 = 1.0 / FPS;

struct CutscenePlayer {
    sprite_sheets: Vec<Texture2D>,
    current_frame: usize,
    total_frames: usize,
    frame_timer: f32,
    is_playing: bool,
    audio: Option<macroquad::audio::Sound>,
}

impl CutscenePlayer {
    async fn new() -> Self {
        let mut sprite_sheets = Vec::new();
        for i in 0..SPRITE_SHEET_COUNT {
            let texture = load_texture(&format!("sheets/sprite_sheet_{:03}.png", i))
                .await
                .unwrap();
            sprite_sheets.push(texture);
        }

        let total_frames = SPRITE_SHEET_COUNT * FRAMES_PER_SHEET;

        let audio = macroquad::audio::load_sound("music.wav").await.ok();

        Self {
            sprite_sheets,
            current_frame: 0,
            total_frames,
            frame_timer: 0.0,
            is_playing: false,
            audio,
        }
    }

    fn update(&mut self, dt: f32) {
        if self.is_playing {
            self.frame_timer += dt;
            if self.frame_timer >= FRAME_TIME {
                self.frame_timer -= FRAME_TIME;
                self.current_frame += 1;
                if self.current_frame >= self.total_frames {
                    self.stop();
                }
            }
        }
    }

    fn draw(&self) {
        if self.is_playing {
            let sheet_index = self.current_frame / FRAMES_PER_SHEET;
            let frame_in_sheet = self.current_frame % FRAMES_PER_SHEET;
            let row = frame_in_sheet / 3;
            let col = frame_in_sheet % 3;

            let src_rect = Rect::new(col as f32 * 600.0, row as f32 * 250.0, 600.0, 250.0);

            let dest_rect = Rect::new(0.0, 0.0, screen_width(), screen_height());

            draw_texture_ex(
                &self.sprite_sheets[sheet_index],
                dest_rect.x,
                dest_rect.y,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(dest_rect.w, dest_rect.h)),
                    source: Some(src_rect),
                    ..Default::default()
                },
            );
        } else {
            clear_background(WHITE);
        }
    }

    fn toggle(&mut self) {
        if self.is_playing {
            self.stop();
        } else {
            self.play();
        }
    }

    fn play(&mut self) {
        self.is_playing = true;
        self.current_frame = 0;
        self.frame_timer = 0.0;
        if let Some(audio) = &self.audio {
            macroquad::audio::play_sound(
                audio,
                macroquad::audio::PlaySoundParams {
                    looped: false,
                    volume: 1.0,
                },
            );
        }
    }

    fn stop(&mut self) {
        self.is_playing = false;
        self.current_frame = 0;
        self.frame_timer = 0.0;
        if let Some(audio) = &self.audio {
            macroquad::audio::stop_sound(audio);
        }
    }
}

#[macroquad::main("Cutscene Player")]
async fn main() {
    let mut player = CutscenePlayer::new().await;

    loop {
        if is_key_pressed(KeyCode::P) {
            player.toggle();
        }

        player.update(get_frame_time());
        player.draw();

        next_frame().await
    }
}
