# twitch
Utility to parse and display data about twitch prime games.

# Usage:
Before any of the other options will work, you must first run:

`twitch --refresh` Extracts data about the available and installed Twitch Prime games from the local Twitch install.

`twitch --list [--installed]` List all of the games available, optionally filtered by whether they are installed or not. Default is to filter by whether they are installed.

`twitch --launcher` Display a launcher based on the most recent twitch metadata. Pressing `enter` shows a full sized version of the game tile. Pressing it again will attempt to launch the game by running the command list in the game's `fuel.json`.

# Limitations
Lots!

* Slow startup
* No way to override the installation location the utility looks for Twitch in.
* Partial support for categorizing a game, but no way to filter the list of games shown in the launcher.
