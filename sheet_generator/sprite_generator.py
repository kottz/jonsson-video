import os
import subprocess
from PIL import Image
import concurrent.futures


def extract_frames(video_path, output_folder):
    os.makedirs(output_folder, exist_ok=True)
    ffmpeg_command = [
        'ffmpeg',
        '-i', video_path,
        '-vf', 'fps=15',
        f'{output_folder}/frame_%04d.png'
    ]
    subprocess.run(ffmpeg_command, check=True)


def create_sprite_sheets(frames_folder, output_folder, sheet_width, sheet_height, frames_per_sheet):
    os.makedirs(output_folder, exist_ok=True)
    frames = sorted([f for f in os.listdir(
        frames_folder) if f.endswith('.png')])

    for sheet_index, i in enumerate(range(0, len(frames), frames_per_sheet)):
        sheet = Image.new('RGB', (sheet_width, sheet_height))
        sheet_frames = frames[i:i+frames_per_sheet]

        for j, frame in enumerate(sheet_frames):
            img = Image.open(os.path.join(frames_folder, frame))
            x = (j % 3) * 600
            y = (j // 3) * 250
            sheet.paste(img, (x, y))

        sheet_path = os.path.join(output_folder, f'sprite_sheet_{
                                  sheet_index:03d}.png')
        sheet.save(sheet_path)


def compress_sheets(sheets_folder, output_folder):
    os.makedirs(output_folder, exist_ok=True)
    for sheet in os.listdir(sheets_folder):
        if sheet.endswith('.png'):
            input_path = os.path.join(sheets_folder, sheet)
            output_path = os.path.join(
                output_folder, sheet.replace('.png', '.webp'))
            cwebp_command = [
                'cwebp',
                input_path,
                '-o', output_path
            ]
            subprocess.run(cwebp_command, check=True)


def compress_sheets_qoi(sheets_folder, output_folder):
    os.makedirs(output_folder, exist_ok=True)
    for sheet in os.listdir(sheets_folder):
        if sheet.endswith('.png'):
            input_path = os.path.join(sheets_folder, sheet)
            output_path = os.path.join(
                output_folder, sheet.replace('.png', '.qoi'))
            qoi_command = [
                'magick',
                input_path,
                output_path
            ]
            subprocess.run(qoi_command, check=True)


def extract_audio(video_path, output_path):
    ffmpeg_command = [
        'ffmpeg',
        '-i', video_path,
        '-vn',
        '-acodec', 'pcm_s16le',
        '-ar', '44100',
        '-ac', '2',
        output_path
    ]
    subprocess.run(ffmpeg_command, check=True)


def process_video(video_path, base_output_folder):
    video_filename = os.path.splitext(os.path.basename(video_path))[0]
    movie_folder = os.path.join(base_output_folder, video_filename)

    frames_folder = os.path.join(movie_folder, 'frames')
    png_sheets_folder = os.path.join(movie_folder, 'sprite_sheets', 'png')
    webp_sheets_folder = os.path.join(movie_folder, 'sprite_sheets', 'webp')
    qoi_sheets_folder = os.path.join(movie_folder, 'sprite_sheets', 'qoi')
    audio_output_path = os.path.join(movie_folder, 'audio.wav')

    sheet_width = 1800
    sheet_height = 2000
    frames_per_sheet = 24  # 3x8 frames per sheet

    extract_frames(video_path, frames_folder)
    create_sprite_sheets(frames_folder, png_sheets_folder,
                         sheet_width, sheet_height, frames_per_sheet)
    compress_sheets(png_sheets_folder, webp_sheets_folder)
    compress_sheets_qoi(png_sheets_folder, qoi_sheets_folder)
    extract_audio(video_path, audio_output_path)


def main():
    input_folder = 'input_videos'  # Folder containing input videos
    base_output_folder = 'movies'

    os.makedirs(base_output_folder, exist_ok=True)

    video_files = [f for f in os.listdir(
        input_folder) if f.endswith(('.mp4', '.avi', '.mov'))]

    with concurrent.futures.ThreadPoolExecutor() as executor:
        futures = []
        for video_file in video_files:
            video_path = os.path.join(input_folder, video_file)
            futures.append(executor.submit(
                process_video, video_path, base_output_folder))

        concurrent.futures.wait(futures)


if __name__ == '__main__':
    main()
