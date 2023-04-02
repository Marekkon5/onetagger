import 'package:flutter/material.dart';
import 'package:json_annotation/json_annotation.dart';
import 'package:onetagger_android/api.dart';
import 'package:path_provider/path_provider.dart';
import 'package:dynamic_color/dynamic_color.dart';
import 'package:path/path.dart' as p;

import 'dart:io';
import 'dart:convert';

part 'settings.g.dart';

Settings settings = Settings();

@JsonSerializable(explicitToJson: true)
class Settings {

  AutoTaggerConfig autoTaggerConfig = AutoTaggerConfig.defaultConfig();

  SpotifyConfig? spotifyConfig;

  bool useMonet = false;

  Settings({
    this.useMonet = false,
    this.spotifyConfig
  });

  /// Get the theme data
  Future<ThemeData> themeData() async {

    // Get color scheme
    var colorScheme = ColorScheme.fromSeed(seedColor: const Color(0xaa00d2bf), brightness: Brightness.dark);
    if (useMonet) {
      var corePalette = await DynamicColorPlugin.getCorePalette();
      if (corePalette != null) {
        colorScheme = corePalette.toColorScheme(brightness: Brightness.dark);
      }
    }

    // Theme data
    return ThemeData(
      colorScheme: colorScheme,
      fontFamily: 'Dosis',
      useMaterial3: true,
      brightness: Brightness.dark
    );

  }

  // Path to settings.json
  static Future<String> _filePath() async {
    return p.join((await getApplicationDocumentsDirectory()).path, 'settings.json');
  }

  // Load from file
  static Future<Settings> load() async {
    try {
      var data = await File(await _filePath()).readAsString();
      return Settings.fromJson(jsonDecode(data));
    } catch (e) {
      return Settings();
    }
  }

  // Save settings to file
  Future<void> save() async {
    var file = File(await _filePath());
    await file.writeAsString(jsonEncode(toJson()));
  }

  factory Settings.fromJson(Map<String, dynamic> json) => _$SettingsFromJson(json);
  Map<String, dynamic> toJson() => _$SettingsToJson(this);

}

class SettingsScreen extends StatefulWidget {
  const SettingsScreen({Key? key}) : super(key: key);

  @override
  State<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends State<SettingsScreen> {
  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Settings', style: const TextStyle(fontWeight: FontWeight.bold),),
        centerTitle: true,
      ),
      body: ListView(
        children: [
          SwitchListTile(
            title: const Text('Use Monet'),
            subtitle: const Text('Use Android 12+ system color palette'),
            value: settings.useMonet,
            secondary: const Icon(Icons.color_lens),
            onChanged: (bool v) async {
              setState(() => settings.useMonet = v);
              await settings.save();
            },
          ),

          ListTile(
            leading: const Icon(Icons.lock_reset, color: Colors.red),
            title: const Text(
              "Reset AutoTagger config",
              style: TextStyle(
                color: Colors.red
              ),
            ),
            subtitle: const Text(
              "Restore the default AutoTagger configuration",
              style: TextStyle(
                color: Colors.red
              ),
            ),
            onTap: () async {
              autoTaggerConfig = AutoTaggerConfig.defaultConfig();
              autoTaggerConfig.applyCustom(jsonDecode(await onetaggerAndroid.customDefault()));
            },
          )

        ],
      ),
    );
  }
}
