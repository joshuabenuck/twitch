# twitch
Utility to list and launch Twitch Prime games.

# Usage:
`twitch --refresh` Extracts data about the available and installed Twitch Prime games from the local Twitch install.

`twitch --list [--installed=true|false]` List all of the games available, optionally filtered by whether they are installed or not.

`twitch --launch <name>` Launch the named game. This is done by either running the command listed in the game's `fuel.json` or opening the twitch URL for the game. The latter is chosen if the `fuel.json` references any Twitch authentication.

# Limitations
* No way to override the installation location the utility looks for Twitch in.
* No fuzzy or partial matching for game names.
* No graphical launcher. If one is desired check out the [doorways](https://github.com/joshuabenuck/doorways) project which builds on this to add graphical launching capabilities.
