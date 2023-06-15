import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:material_design_icons_flutter/material_design_icons_flutter.dart';
import 'package:onetagger_android/settings.dart';
import 'package:onetagger_android/tagging_status.dart';
import 'package:permission_handler/permission_handler.dart';
import 'package:shared_storage/shared_storage.dart';
import 'package:external_path/external_path.dart';
import 'package:flutter/material.dart';
import 'package:onetagger_android/api.dart';

import 'dart:convert';

import 'package:url_launcher/url_launcher.dart';

class AutoTaggerScreen extends StatefulWidget {
  const AutoTaggerScreen({super.key});

  @override
  State<AutoTaggerScreen> createState() => _AutoTaggerScreenState();
}

class _AutoTaggerScreenState extends State<AutoTaggerScreen> {
  
  int page = 0;

  @override
  void initState() {
    super.initState();
  }

  /// Can AT start
  bool canStart() {
    return autoTaggerConfig.path.trim() != '';
  }

  /// Start tagging
  void start() async {
    // Save settings
    autoTaggerConfig.spotify = settings.spotifyConfig;
    settings.autoTaggerConfig = autoTaggerConfig;
    await settings.save();
    // Start
    await onetaggerAndroid.startAt(path: autoTaggerConfig.path, configJson: jsonEncode(autoTaggerConfig.toJson()));
    Navigator.of(context).pop();
    Navigator.of(context).pushReplacement(MaterialPageRoute(builder: (context) => const TaggingStatusScreen()));
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text(
          "AutoTagger",
          style: TextStyle(
            fontWeight: FontWeight.bold
          )
        ),
        centerTitle: true,
      ),
      bottomNavigationBar: NavigationBar(
        onDestinationSelected: (int index) {
          setState(() => page = index);
        },
        destinations: const [
          NavigationDestination(icon: Icon(Icons.web), label: "Platforms"),
          NavigationDestination(icon: Icon(Icons.label), label: "Input & Tags"),
          NavigationDestination(icon: Icon(Icons.person), label: "Platform Specific"),
          NavigationDestination(icon: Icon(Icons.settings), label: "Advanced"),
        ],
        selectedIndex: page,
      ),
      floatingActionButton: (!canStart()) ? null : FloatingActionButton(
        onPressed: () => start(),
        child: const Icon(Icons.play_arrow),
      ),
      body: ([
        const AutoTaggerPlatformsList(),
        AutoTaggerInputTags(updateParent: () => setState(() {})),
        const AutoTaggerPlatformSpecificScreen(),
        const AutoTaggerAdvancedScreen(),
      ])[page]
    );
  }
}

class AutoTaggerPlatformsList extends StatefulWidget {
  const AutoTaggerPlatformsList({Key? key}) : super(key: key);

  @override
  State<AutoTaggerPlatformsList> createState() => _AutoTaggerPlatformsListState();
}

class _AutoTaggerPlatformsListState extends State<AutoTaggerPlatformsList> {

  List<AutoTaggerPlatform> platforms = autoTaggerPlatforms;

  @override
  void initState() {
    super.initState();
  }

  void sort() {
    autoTaggerPlatforms.sort((a, b) {
      var ai = autoTaggerConfig.platforms.indexOf(a.id);
      var bi = autoTaggerConfig.platforms.indexOf(b.id);
      if (ai == -1 && bi == -1) return 0;
      if (ai == -1) return 1;
      if (bi == -1) return -1;
      return ai - bi;
    });
  }

  @override
  Widget build(BuildContext context) {
    return ReorderableListView(
      onReorder: (from, to) {
        if (from > autoTaggerConfig.platforms.length || to > autoTaggerConfig.platforms.length) return;
        var item = autoTaggerConfig.platforms[from];
        autoTaggerConfig.platforms.removeAt(from);

        if (to > from) {
          autoTaggerConfig.platforms.insert(to - 1, item);
        } else {
          autoTaggerConfig.platforms.insert(to, item);
        }

        sort();
        setState(() {});
      },

      children: [
        ...platforms.map((p) => AutoTaggerPlatformCard(p, autoTaggerConfig.platforms.contains(p.id), (bool enable) {
          if (enable) {
            autoTaggerConfig.platforms.add(p.id);
          } else {
            autoTaggerConfig.platforms.remove(p.id);
          }
          sort();
          setState(() {});
        }, key: Key(p.id)))
      ],
    );
  }
}

class AutoTaggerPlatformCard extends StatelessWidget {
  final AutoTaggerPlatform platform;
  final bool isEnabled;
  final Function onChanged;

  const AutoTaggerPlatformCard(this.platform, this.isEnabled, this.onChanged, {Key? key}) : super(key: key);

  IconData speedIcon() {
    if (platform.platform.maxThreads == 1) return MdiIcons.speedometerSlow;
    if (platform.platform.maxThreads == 0) return MdiIcons.speedometer;
    return MdiIcons.speedometerMedium;
  }

  @override
  Widget build(BuildContext context) {
    return Card(
      child: ListTile(
        title: Row(
          mainAxisSize: MainAxisSize.min,
          mainAxisAlignment: MainAxisAlignment.start,
          children: [
            Text(
              platform.platform.name,
              style: const TextStyle(fontWeight: FontWeight.bold),
            ),

            // Speed icon
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 4.0),
              child: Icon(speedIcon(), size: 14.0),
            ),

            // Auth icon
            if (platform.requiresAuth)
              const Padding(
                padding: EdgeInsets.symmetric(horizontal: 4.0),
                child: Icon(
                  Icons.lock,
                  size: 14.0,
                ),
              ),

            // Lyrics icon
            if (platform.supportedTags.contains('unsyncedLyrics'))
              const Padding(
                padding: EdgeInsets.symmetric(horizontal: 4.0),
                child: Icon(
                  Icons.lyrics,
                  size: 14.0
                ),
              )

          ],
        ),
        subtitle: Text(platform.platform.description),
        leading: Image.memory(platform.icon, width: 48.0,),
        trailing: Checkbox(
          value: isEnabled,
          onChanged: (v) {
            onChanged(v);
          },
        ),
      ),
    );
  }
}

class AutoTaggerInputTags extends StatefulWidget {
  final Function? updateParent;

  const AutoTaggerInputTags({this.updateParent, Key? key}) : super(key: key);

  @override
  State<AutoTaggerInputTags> createState() => _AutoTaggerInputTagsState();
}

class _AutoTaggerInputTagsState extends State<AutoTaggerInputTags> {

  String path = '';
  TextEditingController textEditingController = TextEditingController();
  List<AutoTaggerPlatform> platforms = [];

  /// Try to browse for a path
  Future<String> browseFolder() async {
    // Get storage permission
    if (!await Permission.storage.request().isGranted && !await Permission.manageExternalStorage.request().isGranted) {
      throw Exception("Storage permission not granted");
    }
    // Get path from SAF
    var roots = await ExternalPath.getExternalStorageDirectories();
    var uri = await openDocumentTree(grantWritePermission: true);
    if (uri == null) {
      throw Exception("Folder not selected");
    }
    var storage = uri.pathSegments.last.split(":")[0];
    var path = uri.pathSegments.last.split(":")[1];
    if (storage.toLowerCase() == "primary") {
      storage = roots.first;
    } else {
      storage = roots.firstWhere((r) => r.toLowerCase().contains(storage.toLowerCase()));
    }
    path = '$storage/$path';
    return path;
  }

  /// Update the path in config
  updatePath(String path, {bool updateController = true}) {
    this.path = path;
    if (updateController) {
      textEditingController.text = path;
    }
    autoTaggerConfig.path = path;
    setState(() {});
    if (widget.updateParent != null) {
      widget.updateParent!();
    }
  }

  /// Is the tag enabled
  bool isEnabled(String tag) {
    for (var id in autoTaggerConfig.platforms) {
      var platform = autoTaggerPlatforms.firstWhere((p) => p.id == id);
      if (platform.supportedTags.contains(tag)) {
        return true;
      }
    }
    return false;
  }

  @override
  Widget build(BuildContext context) {
    return ListView(
      children: [
        // Path
        Padding(
          padding: const EdgeInsets.symmetric(vertical: 16.0),
          child: Text(
            "Select path:",
            textAlign: TextAlign.center,
            style: TextStyle(
              fontWeight: FontWeight.bold,
              fontSize: 18.0,
              color: Theme.of(context).colorScheme.primary
            ),
          ),
        ),
        Row(
          mainAxisSize: MainAxisSize.max,
          children: [
            Container(width: 8.0),
            SizedBox(
              width: MediaQuery.of(context).size.width - 64,
              child: TextField(
                decoration: const InputDecoration(
                  border: OutlineInputBorder(),
                  isDense: true,
                  hintText: 'Path',
                  labelText: 'Path'
                ),
                controller: textEditingController,
                onChanged: (String text) => updatePath(text, updateController: false),
              ),
            ),
            Container(width: 8.0),
            IconButton(
              icon: const Icon(Icons.open_in_browser, size: 24.0),
              onPressed: () async {
                try {
                  var path = await browseFolder();
                  updatePath(path);
                } catch (e) {
                  ScaffoldMessenger.of(context).showSnackBar(SnackBar(
                    content: Text('Failed obtaining path: $e')
                  ));
                  return;
                }
              },
            )
          ],
        ),


        // Tags
        Padding(
          padding: const EdgeInsets.symmetric(vertical: 16.0),
          child: Text(
            "Select tags:",
            textAlign: TextAlign.center,
            style: TextStyle(
                fontWeight: FontWeight.bold,
                fontSize: 18.0,
                color: Theme.of(context).colorScheme.primary
            ),
          ),
        ),
        GridView.count(
          crossAxisCount: 2,
          physics: const NeverScrollableScrollPhysics(),
          childAspectRatio: (MediaQuery.of(context).size.width / 2.0) / 56.0,
          shrinkWrap: true,
          children: [
            AutoTaggerTag("Album Art", autoTaggerConfig.albumArt, (v) => setState(() => autoTaggerConfig.albumArt = v), isEnabled('albumArt')),
            AutoTaggerTag("Album", autoTaggerConfig.album, (v) => setState(() => autoTaggerConfig.album = v), isEnabled('album')),
            AutoTaggerTag("Album Artist", autoTaggerConfig.albumArtist, (v) => setState(() => autoTaggerConfig.albumArtist = v), isEnabled('albumArtist')),
            AutoTaggerTag("Artist", autoTaggerConfig.artist, (v) => setState(() => autoTaggerConfig.artist = v), isEnabled('artist')),
            AutoTaggerTag("Title", autoTaggerConfig.title, (v) => setState(() => autoTaggerConfig.title = v), isEnabled('title')),
            AutoTaggerTag("Version", autoTaggerConfig.version, (v) => setState(() => autoTaggerConfig.version = v), isEnabled('version')),
            AutoTaggerTag("Remixer", autoTaggerConfig.remixer, (v) => setState(() => autoTaggerConfig.remixer = v), isEnabled('remixer')),
            AutoTaggerTag("Genre", autoTaggerConfig.genre, (v) => setState(() => autoTaggerConfig.genre = v), isEnabled('genre')),
            AutoTaggerTag("Style", autoTaggerConfig.style, (v) => setState(() => autoTaggerConfig.style = v), isEnabled('style')),
            AutoTaggerTag("Label", autoTaggerConfig.label, (v) => setState(() => autoTaggerConfig.label = v), isEnabled('label')),
            AutoTaggerTag("Release ID", autoTaggerConfig.releaseId, (v) => setState(() => autoTaggerConfig.releaseId = v), isEnabled('releaseId')),
            AutoTaggerTag("Track ID", autoTaggerConfig.trackId, (v) => setState(() => autoTaggerConfig.trackId = v), isEnabled('trackId')),
            AutoTaggerTag("BPM", autoTaggerConfig.bpm, (v) => setState(() => autoTaggerConfig.bpm = v), isEnabled('bpm')),
            AutoTaggerTag("Key", autoTaggerConfig.key, (v) => setState(() => autoTaggerConfig.key = v), isEnabled('key')),
            AutoTaggerTag("Mood", autoTaggerConfig.mood, (v) => setState(() => autoTaggerConfig.mood = v), isEnabled('mood')),
            AutoTaggerTag("Catalog Number", autoTaggerConfig.catalogNumber, (v) => setState(() => autoTaggerConfig.catalogNumber = v), isEnabled('catalogNumber')),
            AutoTaggerTag("Track Number", autoTaggerConfig.trackNumber, (v) => setState(() => autoTaggerConfig.trackNumber = v), isEnabled('trackNumber')),
            AutoTaggerTag("Disc Number", autoTaggerConfig.discNumber, (v) => setState(() => autoTaggerConfig.discNumber = v), isEnabled('discNumber')),
            AutoTaggerTag("Duration", autoTaggerConfig.duration, (v) => setState(() => autoTaggerConfig.duration = v), isEnabled('duration')),
            AutoTaggerTag("Track Total", autoTaggerConfig.trackTotal, (v) => setState(() => autoTaggerConfig.trackTotal = v), isEnabled('trackTotal')),
            AutoTaggerTag("ISRC", autoTaggerConfig.isrc, (v) => setState(() => autoTaggerConfig.isrc = v), isEnabled('isrc')),
            AutoTaggerTag("Publish Date", autoTaggerConfig.publishDate, (v) => setState(() => autoTaggerConfig.publishDate = v), isEnabled('publishDate')),
            AutoTaggerTag("Release Date", autoTaggerConfig.releaseDate, (v) => setState(() => autoTaggerConfig.releaseDate = v), isEnabled('releaseDate')),
            AutoTaggerTag("URLs", autoTaggerConfig.url, (v) => setState(() => autoTaggerConfig.url = v), isEnabled('url')),
            AutoTaggerTag("Other", autoTaggerConfig.otherTags, (v) => setState(() => autoTaggerConfig.otherTags = v), isEnabled('otherTags')),
            AutoTaggerTag("OneTagger Tag", autoTaggerConfig.metaTags, (v) => setState(() => autoTaggerConfig.metaTags = v), true),
            AutoTaggerTag("Unsynced Lyrics", autoTaggerConfig.unsyncedLyrics, (v) => setState(() => autoTaggerConfig.unsyncedLyrics = v), isEnabled('unsyncedLyrics')),
            AutoTaggerTag("Synced Lyrics", autoTaggerConfig.syncedLyrics, (v) => setState(() => autoTaggerConfig.syncedLyrics = v), isEnabled('syncedLyrics')),
          ]
        )
      ],
    );
  }
}

class AutoTaggerTag extends StatelessWidget {
  final String title;
  final bool value;
  final bool enabled;
  final Function onChanged;

  const AutoTaggerTag(this.title, this.value, this.onChanged, this.enabled, {Key? key}) : super(key: key);

  @override
  Widget build(BuildContext context) {
    return CheckboxListTile(
      title: Text(title),
      value: value,
      onChanged: (enabled) ? (v) { onChanged(v); } : null,

    );
  }
}

class AutoTaggerPlatformSpecificScreen extends StatefulWidget {
  const AutoTaggerPlatformSpecificScreen({Key? key}) : super(key: key);

  @override
  State<AutoTaggerPlatformSpecificScreen> createState() => _AutoTaggerPlatformSpecificScreenState();
}

class _AutoTaggerPlatformSpecificScreenState extends State<AutoTaggerPlatformSpecificScreen> {

  /// Get the platforms
  List<AutoTaggerPlatform> platforms() {
    return autoTaggerConfig.platforms
      .map((p) => autoTaggerPlatforms.firstWhere((pl) => pl.id == p))
      .where((p) => p.id == 'spotify' || p.platform.customOptions.options.isNotEmpty)
      .toList(growable: false);
  }

  @override
  Widget build(BuildContext context) {
    return ListView(
      children: [
        for (var platform in platforms())
          Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Padding(
                padding: const EdgeInsets.symmetric(vertical: 16.0),
                child: Text(
                  platform.platform.name,
                  textAlign: TextAlign.center,
                  style: TextStyle(
                    fontWeight: FontWeight.bold,
                    fontSize: 18.0,
                    color: Theme.of(context).colorScheme.primary
                  ),
                ),
              ),
              AutoTaggerPlatformSpecificWidget(platform),

              // Spotify override
              if (platform.id == 'spotify')
                SpotifyConfigWidget(spotifyConfig: settings.spotifyConfig),

              Container(height: 16.0),
              const Divider()
            ],
          ),

        // Padding for FAB
        Container(height: 48.0),
      ],
    );
  }
}

class AutoTaggerPlatformSpecificWidget extends StatefulWidget {
  final AutoTaggerPlatform platform;

  const AutoTaggerPlatformSpecificWidget(this.platform, {Key? key}) : super(key: key);

  @override
  State<AutoTaggerPlatformSpecificWidget> createState() => _AutoTaggerPlatformSpecificWidgetState();
}

class _AutoTaggerPlatformSpecificWidgetState extends State<AutoTaggerPlatformSpecificWidget> {

  Widget _optionWidget(PlatformCustomOption option) {
    switch (option.value.type) {
      case PlatformCustomOptionValueType.boolean:
        return PlatformSpecificBooleanWidget(
            option.label,
            autoTaggerConfig.custom[widget.platform.id]![option.id],
            (v) => setState(() => autoTaggerConfig.custom[widget.platform.id]![option.id] = v)
        );
      case PlatformCustomOptionValueType.number:
        return PlatformSpecificNumberWidget(
          option.label, option.value.min!, option.value.max!, option.value.step!,
          autoTaggerConfig.custom[widget.platform.id]![option.id],
          (v) => setState(() => autoTaggerConfig.custom[widget.platform.id]![option.id] = v)
        );
      case PlatformCustomOptionValueType.string:
        return PlatformSpecificStringWidget(
          option.label,
          autoTaggerConfig.custom[widget.platform.id]![option.id],
          (v) => setState(() => autoTaggerConfig.custom[widget.platform.id]![option.id] = v)
        );
      case PlatformCustomOptionValueType.tag:
        // TODO: Implement tag custom option
        return const Text('Unimplemented type: TAG');
      case PlatformCustomOptionValueType.option:
        // TODO: Implement string custom option
        return const Text('Unimplemented type: OPTION');
    }
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        for (var option in widget.platform.platform.customOptions.options)
          _optionWidget(option),
      ],
    );
  }
}

class PlatformSpecificBooleanWidget extends StatelessWidget {
  final String title;
  final bool value;
  final Function onChanged;

  const PlatformSpecificBooleanWidget(this.title, this.value, this.onChanged, {Key? key}) : super(key: key);

  @override
  Widget build(BuildContext context) {
    return SwitchListTile(
      title: Text(title),
      value: value,
      onChanged: (v) => onChanged(v)
    );
  }
}

class PlatformSpecificNumberWidget extends StatelessWidget {
  final String title;
  final int min;
  final int max;
  final int step;
  final int value;
  final Function onChanged;

  const PlatformSpecificNumberWidget(this.title, this.min, this.max, this.step, this.value, this.onChanged, {Key? key}) : super(key: key);

  @override
  Widget build(BuildContext context) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        Padding(
          padding: const EdgeInsets.symmetric(vertical: 4.0),
          child: Text(
            title,
            style: const TextStyle(
              fontSize: 16.0
            )
          ),
        ),
        Slider(
          min: min.toDouble(),
          max: max.toDouble(),
          divisions: ((max - min) / step).round(),
          label: value.toString(),
          value: value.toDouble(),
          onChanged: (double v) => onChanged(v.round()),
        )
      ],
    );
  }
}

class PlatformSpecificStringWidget extends StatefulWidget {
  final String title;
  final String value;
  final Function onChanged;

  const PlatformSpecificStringWidget(this.title, this.value, this.onChanged, {Key? key}) : super(key: key);

  @override
  State<PlatformSpecificStringWidget> createState() => _PlatformSpecificStringWidgetState();
}

class _PlatformSpecificStringWidgetState extends State<PlatformSpecificStringWidget> {

  TextEditingController controller = TextEditingController();

  @override
  void initState() {
    controller.text = widget.value;
    super.initState();
  }

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(8.0),
      child: TextField(
        controller: controller,
        decoration: InputDecoration(
          border: const OutlineInputBorder(),
          hintText: widget.title,
          labelText: widget.title,
          isDense: true,
        ),
        onChanged: (String v) {
          widget.onChanged(v);
        },
      ),
    );
  }
}


class AutoTaggerAdvancedScreen extends StatefulWidget {
  const AutoTaggerAdvancedScreen({Key? key}) : super(key: key);

  @override
  State<AutoTaggerAdvancedScreen> createState() => _AutoTaggerAdvancedScreenState();
}

class _AutoTaggerAdvancedScreenState extends State<AutoTaggerAdvancedScreen> {
  @override
  Widget build(BuildContext context) {
    return ListView(
      children: [
        SwitchListTile(
          title: const Text('Overwrite tags'),
          value: autoTaggerConfig.overwrite,
          onChanged: (v) => setState(() => autoTaggerConfig.overwrite = v),
        ),
        SwitchListTile(
          title: const Text('ID3v2.4'),
          value: autoTaggerConfig.id3v24,
          onChanged: (v) => setState(() => autoTaggerConfig.id3v24 = v),
        ),
        SwitchListTile(
          title: const Text('Short title'),
          value: autoTaggerConfig.shortTitle,
          onChanged: (v) => setState(() => autoTaggerConfig.shortTitle = v),
        ),
        SwitchListTile(
          title: const Text('Save album art to file'),
          value: autoTaggerConfig.albumArtFile,
          onChanged: (v) => setState(() => autoTaggerConfig.albumArtFile = v),
        ),
        SwitchListTile(
          title: const Text('Merge/append genres & styles'),
          value: autoTaggerConfig.mergeGenres,
          onChanged: (v) => setState(() => autoTaggerConfig.mergeGenres = v),
        ),
        SwitchListTile(
          title: const Text('Camelot key notation'),
          value: autoTaggerConfig.camelot,
          onChanged: (v) => setState(() => autoTaggerConfig.camelot = v),
        ),
        SwitchListTile(
          title: const Text('Use track or release ID as input to get exact match'),
          value: autoTaggerConfig.matchById,
          onChanged: (v) => setState(() => autoTaggerConfig.matchById = v),
        ),
        SwitchListTile(
          title: const Text('Identify tracks with Shazam'),
          value: autoTaggerConfig.enableShazam,
          onChanged: (v) => setState(() => autoTaggerConfig.enableShazam = v),
        ),
        if (autoTaggerConfig.enableShazam)
          SwitchListTile(
            title: const Text('Force Shazam'),
            value: autoTaggerConfig.forceShazam,
            onChanged: (v) => setState(() => autoTaggerConfig.forceShazam = v),
          ),
        SwitchListTile(
          title: const Text('Skip already tagged tracks'),
          value: autoTaggerConfig.skipTagged,
          onChanged: (v) => setState(() => autoTaggerConfig.skipTagged = v),
        ),
        SwitchListTile(
          title: const Text('Include subfolders'),
          value: autoTaggerConfig.includeSubfolders,
          onChanged: (v) => setState(() => autoTaggerConfig.includeSubfolders = v),
        ),
        SwitchListTile(
          title: const Text('Only write year'),
          value: autoTaggerConfig.onlyYear,
          onChanged: (v) => setState(() => autoTaggerConfig.onlyYear = v),
        ),
        SwitchListTile(
          title: const Text('Tag same track on multiple platforms'),
          value: autoTaggerConfig.multiplatform,
          onChanged: (v) => setState(() => autoTaggerConfig.multiplatform = v),
        ),
        SwitchListTile(
          title: const Text('Write .LRC file'),
          value: autoTaggerConfig.writeLrc,
          onChanged: (v) => setState(() => autoTaggerConfig.writeLrc = v),
        ),
        const Padding(
          padding: EdgeInsets.symmetric(vertical: 8.0, horizontal: 16.0),
          child: Text(
            'Strictness',
            style: TextStyle(
              fontSize: 16.0
            ),
          ),
        ),
        Slider(
          min: 0.0,
          max: 1.0,
          divisions: 20,
          value: autoTaggerConfig.strictness,
          label: 'Strictness: ${(autoTaggerConfig.strictness * 100.0).round()}%',
          onChanged: (double v) => setState(() => autoTaggerConfig.strictness = v),
        ),
        const Padding(
          padding: EdgeInsets.symmetric(vertical: 8.0, horizontal: 16.0),
          child: Text(
            'Search Threads',
            style: TextStyle(
              fontSize: 16.0
            ),
          ),
        ),
        Slider(
          min: 0.0,
          max: 16.0,
          divisions: 16,
          value: autoTaggerConfig.threads.toDouble(),
          label: 'Search Threads: ${autoTaggerConfig.threads}',
          onChanged: (double v) => setState(() => autoTaggerConfig.threads = v.round()),
        ),

        Padding(
          padding: const EdgeInsets.symmetric(vertical: 16.0),
          child: Text(
            "Separators:",
            textAlign: TextAlign.center,
            style: TextStyle(
                fontWeight: FontWeight.bold,
                fontSize: 18.0,
                color: Theme.of(context).colorScheme.primary
            ),
          ),
        ),
        TagSeparatorsWidget(autoTaggerConfig.separators, (TagSeparators separators) => autoTaggerConfig.separators = separators),
        Container(height: 48.0),
      ],
    );
  }
}

class TagSeparatorsWidget extends StatefulWidget {
  final TagSeparators tagSeparators;
  final Function onChanged;

  const TagSeparatorsWidget(this.tagSeparators, this.onChanged, {Key? key}) : super(key: key);

  @override
  State<TagSeparatorsWidget> createState() => _TagSeparatorsWidgetState();
}

class _TagSeparatorsWidgetState extends State<TagSeparatorsWidget> {

  TagSeparators tagSeparators = TagSeparators();
  TextEditingController id3Controller = TextEditingController();
  TextEditingController vorbisController = TextEditingController();
  TextEditingController mp4Controller = TextEditingController();

  @override
  void initState() {
    // Initial text
    tagSeparators = widget.tagSeparators;
    id3Controller.text = widget.tagSeparators.id3;
    vorbisController.text = widget.tagSeparators.vorbis??'';
    mp4Controller.text = widget.tagSeparators.mp4;

    super.initState();
  }

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        // ID3
        Container(width: 8.0),
        Flexible(
          child: TextField(
            controller: id3Controller,
            onChanged: (String v) {
              tagSeparators.id3 = v;
              widget.onChanged(tagSeparators);
            },
            decoration: const InputDecoration(
              border: OutlineInputBorder(),
              hintText: 'ID3 (MP3 + AIFF)',
              labelText: 'ID3 (MP3 + AIFF)',
              isDense: true
            ),
          ),
        ),
        Container(width: 8.0),

        // Vorbis
        Container(width: 8.0),
        Flexible(
          child: TextField(
            controller: vorbisController,
            onChanged: (String v) {
              tagSeparators.vorbis = v;
              widget.onChanged(tagSeparators);
            },
            decoration: const InputDecoration(
              border: OutlineInputBorder(),
              hintText: 'Vorbis (FLAC + OGG)',
              labelText: 'Vorbis (FLAC + OGG)',
              isDense: true
            ),
          ),
        ),
        Container(width: 8.0),

        // MP4
        Container(width: 8.0),
        Flexible(
          child: TextField(
            controller: mp4Controller,
            onChanged: (String v) {
              tagSeparators.mp4 = v;
              widget.onChanged(tagSeparators);
            },
            decoration: const InputDecoration(
              border: OutlineInputBorder(),
              hintText: 'MP4 (M4A)',
              labelText: 'MP4 (M4A)',
              isDense: true
            ),
          ),
        ),
        Container(width: 8.0),

      ],
    );
  }
}

class SpotifyConfigWidget extends StatefulWidget {
  final SpotifyConfig? spotifyConfig;

  const SpotifyConfigWidget({this.spotifyConfig, Key? key}) : super(key: key);

  @override
  State<SpotifyConfigWidget> createState() => _SpotifyConfigWidgetState();
}

class _SpotifyConfigWidgetState extends State<SpotifyConfigWidget> {

  String clientId = '';
  String clientSecret = '';
  TextEditingController clientIdController = TextEditingController();
  TextEditingController clientSecretController = TextEditingController();
  bool authorized = false;

  @override
  void initState() {
    clientId = widget.spotifyConfig?.clientId??'';
    clientSecret = widget.spotifyConfig?.clientSecret??'';
    clientIdController.text = clientId;
    clientSecretController.text = clientSecret;

    super.initState();
  }

  void login() async {
    try {
      var url = await onetaggerAndroid.authorizeSpotify(clientId: clientId, clientSecret: clientSecret);
      // Authorized
      if (url == null) {
        setState(() => authorized = true);
        settings.spotifyConfig = SpotifyConfig(clientId, clientSecret);
        await settings.save();
        return;
      }
      // Open in browser
      launchUrl(Uri.parse(url), mode: LaunchMode.externalApplication);
    } catch (e) {
      ScaffoldMessenger.of(context).showSnackBar(SnackBar(
        content: Text('Failed authorizing Spotify: $e'),
      ));
    }
  }

  @override
  Widget build(BuildContext context) {
    if (authorized) {
      return Row(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          const Icon(Icons.done, color: Colors.green, size: 20.0),
          Container(width: 8.0),
          const Text(
            "Spotify Authorized!",
            style: TextStyle(
              fontSize: 16.0
            ),
          )
        ],
      );
    }

    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        // Guide
        RichText(
          textAlign: TextAlign.center,
          text: TextSpan(
            text: '',
            style: DefaultTextStyle.of(context).style,
            children: [
              const TextSpan(text: '1. Open '),
              TextSpan(
                text: 'Spotify Developer',
                style: TextStyle(color: Theme.of(context).colorScheme.primary),
                recognizer: TapGestureRecognizer()..onTap = () => launchUrl(Uri.parse('https://developer.spotify.com/'), mode: LaunchMode.externalApplication)
              ),
              const TextSpan(text: ' account & create an app\n'),
              const TextSpan(text: '2. In settings set the Callback URL to: '),
              TextSpan(
                text: 'http://127.0.0.1:36914/spotify',
                style: TextStyle(color: Theme.of(context).colorScheme.primary),
                recognizer: TapGestureRecognizer()..onTap = () async {
                  await Clipboard.setData(const ClipboardData(text: 'http://127.0.0.1:36914/spotify'));
                  ScaffoldMessenger.of(context).showSnackBar(const SnackBar(content: Text('Copied!')));
                }
              ),
              const TextSpan(text: '\n'),
              const TextSpan(text: '3. Enter your Client ID & Client Secret below & click Login\n\n'),
              TextSpan(
                text: 'Video Tutorial',
                style: TextStyle(color: Theme.of(context).colorScheme.primary),
                recognizer: TapGestureRecognizer()..onTap = () => launchUrl(Uri.parse('https://www.youtube.com/watch?v=i0q5qWQSH9Y'), mode: LaunchMode.externalApplication)
              )
            ]
          )
        ),
        Container(height: 32.0),

        // Login fields
        Row(
          children: [
            Container(width: 8.0),
            Flexible(
              child: TextField(
                controller: clientIdController,
                onChanged: (String v) {
                  clientId = v;
                },
                decoration: const InputDecoration(
                  border: OutlineInputBorder(),
                  hintText: 'Client ID',
                  labelText: 'Client ID',
                  isDense: true
                ),
              ),
            ),
            Container(width: 8.0),
            Flexible(
              child: TextField(
                controller: clientSecretController,
                onChanged: (String v) {
                  clientSecret = v;
                },
                decoration: const InputDecoration(
                  border: OutlineInputBorder(),
                  hintText: 'Client Secret',
                  labelText: 'Client Secret',
                  isDense: true
                ),
              ),
            ),
            Container(width: 8.0),
            ElevatedButton(
              onPressed: () => login(),
              child: const Text('Login'),
            ),
            Container(width: 8.0),
          ],
        )

      ],
    );
  }
}
