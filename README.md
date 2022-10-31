# simple-subtitle-sync

## Usage:
subsync.exe [-if subtitle framerate] [-of video framerate] [-o output file] -i input file
If not specified, a frame rate of 29,97 is assumed, and the resulting subtitle will be called output.srt

## How it works:
The program loads the given .srt file into memory

Next it looks for strings matching the .srt timestamp format (hh:mm:ss,ms).

Every match is converted into miliseconds, then multiplied by input framerate to get absolute frames.

Next, the frames are divided by the video framerate, and rebuilt to the hh:mm:ss,ms format.

Finally the result is written to the output file.
