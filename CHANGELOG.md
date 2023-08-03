# 1.7.0
**(03.08.2023)**

### Auto Tag:
- Updated Beatport to support the new site
- Improved match rates
- Selectable overwrite
- Internal refactor to how tags work
- Explicit tag
- BPMSupreme > Latino

### Quick Tag:
- **Multiple files mode**
- Thin view mode
- External player support

### Renamer:
- Fixed input updating
- BPM in MP4

### Other:
- Various bug fixes and improvements in all sections


# 1.6.0
**(14.04.2023)**

### OneTagger:
- Support for WAV (ID3) and OGG files

### Auto Tag:
- **Android version**
- Added Bandcamp
- Added Musixmatch
- Added Deezer
- Lyrics support
- Logging in custom platforms
- Stop button
- Better platform info (supported tags, whether auth is required)
- Profiles (multiple configurations)

### Other
- Many bug fixes in all sections
- Dropped dependency on libsndfile, so compilling 1T should be less painful and more portable


# 1.5.1
**(31.10.2022)**

### OneTagger:
- Older MacOS warning and server version restart

### Auto Tag:
- HOTFIX: Make sure the Move files feature is more safe


# 1.5.0
**(14.10.2022)**

### OneTagger:
- **Rewrite entire frontend to Vue3 + Typescript**

### Auto Tag:
- Only write year
- Track/disc number/total tags
- BPMSupreme, iTunes improvements
- Tag each track using multiple platforms
- Merging styles and genres fixes
- Regex title cleanup
- Move failed/successful files

### Audio Features:
- Spofiy rate limits
- Bug fixes

### Quick Tag:
- List of failed files with reasons


# 1.4.0
**(20.04.2022)**

### OneTagger:
- Added Auto Rename tab (rename your files by to your tags)
- CLI version

### Auto Tag:
- Internal platform system redesign
- Custom platforms support (https://github.com/Marekkon5/onetagger-platform-template/)
- Update Spotify implementation
- Improved match rates, less skipped files
- Various minor platform fixes
- Added reason of failure into status list

### Quick Tag:
- Redesign
- Sorting
- Multiple genres
- File browser
- Subgenres


### Other:
- Windows install to registry
- New logging system
- Split to bunch of different crates
- Minor improvements and bug fixes


# 1.3.0:
**(18.11.2021)**

### Auto Tag:
 - Match by exact ID for Discogs, Beatport
 - Filename template fixes
 - Duration tag
 - `VINYLTRACK` Tag for Discogs
 - Discogs now faster for smaller batches
 - Album artist tag
 - iTunes, Musicbrainz, Beatsource and Spotify support
 - Beatport subgenres, more tags
 - Meta tag
 - Remixer tag
 - Track number tag
 - ISRC tag
 - Shazam to find songs without tag and filename parsing
 - Filter in status page

### Audio Features:
 - Added popularity tag
 - Renamed danceability value to `dance-high, dance-med, dance-low`

### Quick Tag:
 - Internal rewrite, cleaner code, more stable
 - Search and filter

### Tag Editor:
 - CTRL + S
 - Filtering
 - Refactored some code

### Other:
 - General UI improvements
 - Windows: Replace CEF with webview2 - smaller install sizes, more portable.
 - `--expose` command line option to make the server listen on 0.0.0.0
 - Updated dependencies
 - Bug fixes
 - .mp4 Extension support


# 1.2.1:
**(03.07.2021)**

### Auto Tagger
- Fixed bug in 1.2.0 causing Beatport and Traxsource having low match rate

### Other
- Added more info to logs for debugging
- Fixed path pickers not opening with bad path


# 1.2.0
**(02.07.2021)**

### Shared:
- Added M3U playlist support with drag and drop

### Auto Tag:
- Added catalog number, track ID, release ID, version, URL tags
- Added duration matching (WARNING: strict, should be used only in specific situations)
- Tag files without metadata (using filename with custom templates)
- Single page design changes
- Improved matching rates, bug fixes

### Quick Tag:
- Option to load recursively

### Tag Editor:
- Minor design changes

### Other
- Benchmark mode (for testing / debugging purproses, can be ran with `--benchmark` command line argument)
- If you specify path as command line argument, it will be automatically prefilled.



# 1.1.0
**(31.05.2021)**

### Shared:
- Added MP4/M4A support

### Auto Tag:
- Redesign
- Better status page
- Single page setup (enable in settings)
- Camelot key notation
- Juno Download is single thread now
- Bug fixes and improvements related to matching

### Audio Features:
- Fix searching in some edge cases
- Cache Spotify token

### Quick Tag:
- Player UI improvements
- Autosave, Autoplay (can be enabled in settings)
- Bug fixes

## **WARNING: Due to many breaking changes, SETTINGS WILL BE RESET TO DEFAULT. This will hopefully not happen in future again. Sorry for the inconvenience.**



# 1.0.0
**(13.05.2021)**

First public release
