# OneTagger Custom Platforms

This is a guide on how to create your own platforms for OneTagger and also publish them into the repository for others to use (https://github.com/Marekkon5/onetagger-platforms)

Custom platforms can be written either in Python or Rust.

## Useful info

- In OneTagger you can access the developer tools using the `F3` key which allow you to see the docs, view log, view versions or quickly reload the app.

- Locations of custom platforms are:
    - Linux: `~/.config/onetagger/platforms`
    - Windows: `%appdata%\OneTagger\OneTagger\platforms`
    - MacOS: `/Users/your-user-account/Library/Preferences/com.OneTagger.OneTagger/platforms`

- Python platforms are folders, Rust platforms are either `.so`, or `.dylib` or `.dll`

- Links:
    - Repo: https://github.com/Marekkon5/onetagger-platforms
    - Rust Template: https://github.com/Marekkon5/onetagger-platform-template


- Languages:
    - Python:
      - +Easier to write
      - +Should be more compatible across versions
      - +Library ecosystem
      - -Slower
      - -You can't share data across threads (they're subprocesses)
    
    - Rust:
      - +They're same as "native" 1T platforms
      - +Faster
      - -Harder to write
      - -Only compatible with one version of 1T

# Python

## 0. Setup
To begin you should create a folder which should be your "workspace". It is recommended to create that folder inside 1T platforms folder for easy testing.

You should also open 1T DevTools (F3 key) to check the current versions, and to generate Python documentation.

## 1. Manifest

Create `info.json` file and add the following:

```json
{
    "main": "main.py",
    "requirements": [],
    "id": "example",
    "name": "Python Platform",
    "description": "Python Platform Example",
    "version": "1.0.0",
    "maxThreads": 1,
    "supportedVersion": 42,
    "author": "marekkon5",
    "customOptions": {
        "options": []
    },
    "supportedTags": [
        "title", "artist"
    ],
    "requiresAuth": true
}
```

- `main` - should be name of your `.py` file
- `requirements` - pip requirements to install, same format as `pip install`
- `id` - should be globally unique, lowercase with underscores
- `supportedVersion` - you can find it in devtools (`F3` in OneTagger)
- `supportedTags` - camelCase version of `SupportedTag` inside the documentation

### 1.1 Custom Options:

All options need `id` and `label`, and optionally you can add `tooltip` to them.

`id` should be unique and you can use it to access the config values later.

#### String (Text field)
`hidden` field is optional, if true, the value is hidden like a password.

```json 
{
    "id": "option_id",
    "label": "Option Label",
    "value": {
        "type": "string",
        "value": "Initial Value",
        "hidden": "false"
    }
}
```

#### Boolean (Switch)
```json
{
    "id": "option_id",
    "label": "Option Label",
    "value": {
        "type": "boolean",
        "value": false
    }
}

```

#### Number (Slider)
```json
{
    "id": "option_id",
    "label": "Option Label",
    "value": {
        "type": "number",
        "value": 0,
        "min": 0,
        "max": 100,
        "step": 1
    }
}
```

#### Button
Function `config_callback`  will be called with the id and current config.

```json
{
    "id": "option_id",
    "label": "Option Label",
    "value": {
        "type": "button"
    }
}
```

#### Option (Dropdown)
```json
{
    "id": "option_id",
    "label": "Option Label",
    "value": {
        "type": "option",
        "value": "A",
        "values": ["A", "B", "C"]
    }
}
```

#### Tag
```json
{
    "id": "option_id",
    "label": "Option Label",
    "value": {
        "type": "tag",
        "value": {
            "id3": "FRAMENAME",
            "vorbis": "FRAMENAME",
            "mp4": "FRAMENAME"
        }
    }
}
```


# 2. Platform

Create a `.py` file named like your `main` field inside `info.json`

## Minimal template:

```python
from onetagger import AudioFileInfo, TaggerConfig, TrackMatch, Track

def match_track(info: AudioFileInfo, config: TaggerConfig) -> list[TrackMatch]:
    return []

def extend_track(track: Track, config: TaggerConfig) -> Track:
    return track
```

`match_track` - Should search for track and return all matches with minimal info
`extend_track` - Gets called on matched track afterwards to fill any remaining info

## Commented example

```python
from onetagger import AudioFileInfo, TaggerConfig, TrackMatch, Track, new_track, match_tracks

# This function should search on your platform and return list of relevant / matched tracks
def match_track(info: AudioFileInfo, config: TaggerConfig) -> list[TrackMatch]:
    # Here implement searching in your API
    ...
    # Now convert your tracks into 1T Tracks
    track = new_track(
        platform = "your_platform",
        title = "Track title",
        artists = ["Artist"]
    )
    # Match your tracks
    matches = match_tracks(info, [track], config, True)
    return matches

# This function will be called later on matched track, so here you can fetch additional metadata
def extend_track(track: Track, config: TaggerConfig) -> Track:
    track.album_artists = ["Example"]
    return track

# Optional
# This function is called if user presses a button inside the platform config step
# `id` of the button here is name
# You can either return nothing (to do nothing), or return a dict which will update the config
def config_callback(name: str, config):
    return { "option_id": "value" }
```

## 3. Icon (optional)

You can place a `icon.png` file inside your project folder. It should be square and sized around 50x50. 

## 4. Publishing (optional)

If you wish to make this platform accessible to others via the Platforms Repo inside OneTagger, do the following:

1. Fork https://github.com/Marekkon5/onetagger-platforms
2. Clone your fork
3. Add your code inside `platforms/your_platform_id`
4. Push
5. Create Pull Request at https://github.com/Marekkon5/onetagger-platforms
6. Once merged, CI should do all the manifest updating.



# Rust

## 0. Setup
- You should always use the latest stable version of Rust.  
- Rust does NOT have a stable ABI, and OneTagger is constantly getting updates, so your platform will work only with specific 1T version ("custom platform compatibility" number in DevTools)
- You can get the docs by running `cargo doc --open` inside your project
- All built-in 1T platforms are same as Rust custom platforms, so for examples / inspiration you can check the 1T source code.

## 1. Clone the template repository
```
git clone https://github.com/Marekkon5/onetagger-platform-template.git my-platform
```

## 2. Edit `Cargo.toml`

Change `my-platform` to your platform name
```
[package]
name = "my-platform"
```

Add dependencies:

```
[dependencies]
log = "0.4"
anyhow = "1.0"
serde_json = "1.0"

onetagger-tagger = { git = "https://github.com/Marekkon5/onetagger.git" }
```

**NOTE:** It is recommended to use a specific commit of `onetagger-tagger`

## 3. Implement
In `src/lib.rs` implement an `AutotaggerSourceBuilder`, `AutotaggerSource` and call `create_plugin!` with your `AutotaggerSourceBuilder` implementation.

The existing code includes barebone example implementation that doesn't match any track.

To see docs run: `cargo doc --open`

## 4. Replace the `icon.png` with your own.
 (Should be 1:1 aspect ratio)

## 5. Compile
```
cargo build --release
```
(Or you can push to Github and wait for the Github Actions CI to build your plugin to all platforms)

## 6. Install:

From `target/release/` copy:
- on Linux: `.so` file
- on Window: `.dll` file
- on MacOS: `.dylib` file

to: 
- on Linux: `~/.config/onetagger/platforms`
- on Window: `%appdata%\OneTagger\OneTagger\platforms`
- on MacOS: `/Users/your-user-account/Library/Preferences/com.OneTagger.OneTagger/platforms`


## 7. Restart OneTagger and use.

## 8. (Optional) Publish

You can create or edit the `info.json` file with metadata about the platform, and then create a Pull Request to include this platform within the `platforms` folder in https://github.com/Marekkon5/onetagger-platforms

By doing this other will be able to download your platform directly from 1T.