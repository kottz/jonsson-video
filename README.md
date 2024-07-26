# jonsson-video

This repo contains some experimentation to playing video cutscenes with macroquad.

There is an extractor script which takes the input video files and generates sprite sheets of the frames. These are then loaded and rendered with macroquad.
Just copy the videos from the disc in the `data/mov` folder into `sheet_generator/input_videos` and run the script.

This way of doing it isn't very efficient but the original videos are 15 fps at 600x250 so it works well enough without taking up a huge amount of space.
I would rather my assets be a few MB larger than going through the hassle of bundling a h264 decoder.

The extractor requires `ffmpeg` and `libwebp` to be installed.
