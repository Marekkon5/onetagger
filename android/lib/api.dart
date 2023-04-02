import 'dart:convert';
import 'dart:ffi';
import 'dart:typed_data';
import 'package:json_annotation/json_annotation.dart';
import 'package:onetagger_android/api_generated.dart';

part 'api.g.dart';

OnetaggerAndroid onetaggerAndroid =
    OnetaggerAndroidImpl(DynamicLibrary.open("libonetagger_android.so"));

/// Global AT config
AutoTaggerConfig autoTaggerConfig = AutoTaggerConfig.defaultConfig();
/// Global platforms list
late List<AutoTaggerPlatform> autoTaggerPlatforms;

/// Autotagger config
@JsonSerializable(explicitToJson: true)
class AutoTaggerConfig {
  AutoTaggerConfig({
    required this.album,
    required this.albumArt,
    required this.albumArtFile,
    required this.albumArtist,
    required this.artist,
    required this.bpm,
    required this.camelot,
    required this.catalogNumber,
    required this.custom,
    required this.discNumber,
    required this.duration,
    required this.enableShazam,
    required this.enhancedLrc,
    required this.filenameTemplate,
    required this.forceShazam,
    required this.genre,
    required this.id3v24,
    required this.includeSubfolders,
    required this.isrc,
    required this.key,
    required this.label,
    required this.matchById,
    required this.matchDuration,
    required this.maxDurationDifference,
    required this.mergeGenres,
    required this.metaTags,
    required this.mood,
    required this.moveFailed,
    required this.moveSuccess,
    required this.multiplatform,
    required this.multipleMatches,
    required this.onlyYear,
    required this.otherTags,
    required this.overwrite,
    required this.parseFilename,
    required this.path,
    required this.platforms,
    required this.publishDate,
    required this.releaseDate,
    required this.releaseId,
    required this.remixer,
    required this.separators,
    required this.shortTitle,
    required this.skipTagged,
    required this.strictness,
    required this.style,
    required this.stylesCustomTag,
    required this.stylesOptions,
    required this.syncedLyrics,
    required this.threads,
    required this.title,
    required this.trackId,
    required this.trackNumber,
    required this.trackNumberLeadingZeroes,
    required this.trackTotal,
    required this.unsyncedLyrics,
    required this.url,
    required this.version,
    required this.writeLrc,
    this.titleRegex,
    this.moveFailedPath,
    this.moveSuccessPath
  });

  bool album;
  bool albumArt;
  bool albumArtFile;
  bool albumArtist;
  bool artist;
  bool bpm;
  bool camelot;
  bool catalogNumber;
  Map<String, Map<String, dynamic>> custom;
  bool discNumber;
  bool duration;
  bool enableShazam;
  bool enhancedLrc;
  String? filenameTemplate;
  bool forceShazam;
  bool genre;
  bool id3v24;
  bool includeSubfolders;
  bool isrc;
  bool key;
  bool label;
  bool matchById;
  bool matchDuration;
  int maxDurationDifference;
  bool mergeGenres;
  bool metaTags;
  bool mood;
  bool moveFailed;
  bool moveSuccess;
  bool multiplatform;
  String multipleMatches;
  bool onlyYear;
  bool otherTags;
  bool overwrite;
  bool parseFilename;
  String path;
  List<String> platforms;
  bool publishDate;
  bool releaseDate;
  bool releaseId;
  bool remixer;
  TagSeparators separators;
  bool shortTitle;
  bool skipTagged;
  double strictness;
  bool style;
  FrameName? stylesCustomTag;
  String stylesOptions;
  bool syncedLyrics;
  int threads;
  bool title;
  bool trackId;
  bool trackNumber;
  int trackNumberLeadingZeroes;
  bool trackTotal;
  bool unsyncedLyrics;
  bool url;
  bool version;
  bool writeLrc;
  String? titleRegex;
  String? moveSuccessPath;
  String? moveFailedPath;
  SpotifyConfig? spotify;

  factory AutoTaggerConfig.defaultConfig() => AutoTaggerConfig(
    platforms: ["beatport"],
    path: "",
    title: false,
    artist: false,
    album: false,
    key: false,
    bpm: true,
    genre: true,
    style: true,
    label: true,
    releaseDate: true,
    publishDate: false,
    albumArt: false,
    otherTags: false,
    catalogNumber: false,
    url: false,
    trackId: false,
    releaseId: false,
    version: false,
    duration: false,
    albumArtist: false,
    remixer: false,
    trackNumber: false,
    trackTotal: false,
    discNumber: false,
    isrc: false,
    mood: false,
    syncedLyrics: false,
    unsyncedLyrics: false,
    metaTags: false,
    separators: TagSeparators(),
    id3v24: true,
    overwrite: true,
    threads: 16,
    strictness: 0.7,
    mergeGenres: false,
    albumArtFile: false,
    camelot: false,
    parseFilename: false,
    filenameTemplate: "%artists% - %title%",
    shortTitle: false,
    matchDuration: false,
    maxDurationDifference: 30,
    matchById: false,
    multipleMatches: "Default",
    stylesOptions: "default",
    stylesCustomTag: FrameName.same("STYLE"),
    trackNumberLeadingZeroes: 0,
    enableShazam: false,
    forceShazam: false,
    skipTagged: false,
    includeSubfolders: true,
    onlyYear: false,
    titleRegex: null,
    moveSuccess: false,
    moveSuccessPath: null,
    moveFailed: false,
    moveFailedPath: null,
    writeLrc: false,
    enhancedLrc: false,
    multiplatform: false,
    custom: {},
  );

  /// Apply custom config
  applyCustom(Map<String, dynamic> custom) {
    for (var platform in custom.keys) {
      // All keys
      if (!this.custom.containsKey(platform)) {
        this.custom[platform] = custom[platform];
        continue;
      }
      // Per key restore
      for (var key in custom[platform].keys) {
        if (!this.custom[platform]!.containsKey(key)) {
          this.custom[platform]![key] = custom[platform][key];
        }
      }
    }
  }

  factory AutoTaggerConfig.fromJson(Map<String, dynamic> json) => _$AutoTaggerConfigFromJson(json);
  Map<String, dynamic> toJson() => _$AutoTaggerConfigToJson(this);
}

@JsonSerializable(explicitToJson: true)
class FrameName {
  String id3;
  String vorbis;
  String mp4;

  FrameName(this.id3, this.vorbis, this.mp4);

  factory FrameName.same(String tag) {
    return FrameName(tag, tag, tag);
  }
  
  factory FrameName.fromJson(Map<String, dynamic> json) => _$FrameNameFromJson(json);
  Map<String, dynamic> toJson() => _$FrameNameToJson(this);
}

@JsonSerializable(explicitToJson: true)
class TagSeparators {
  String id3;
  String? vorbis;
  String mp4;

  TagSeparators({ this.id3 = ", ", this.vorbis, this.mp4 = ", " });

  factory TagSeparators.fromJson(Map<String, dynamic> json) => _$TagSeparatorsFromJson(json);
  Map<String, dynamic> toJson() => _$TagSeparatorsToJson(this);
}

@JsonSerializable(explicitToJson: true)
class SpotifyConfig {
  String clientId;
  String clientSecret;

  SpotifyConfig(this.clientId, this.clientSecret);

  factory SpotifyConfig.fromJson(Map<String, dynamic> json) => _$SpotifyConfigFromJson(json);
  Map<String, dynamic> toJson() => _$SpotifyConfigToJson(this);
}

@JsonSerializable(explicitToJson: true)
class TaggingStatusWrap {
  TaggingStatus status;
  String platform;
  double progress;

  TaggingStatusWrap(this.status, this.platform, this.progress);

  factory TaggingStatusWrap.fromJson(Map<String, dynamic> json) => _$TaggingStatusWrapFromJson(json);
  Map<String, dynamic> toJson() => _$TaggingStatusWrapToJson(this);
}

@JsonSerializable(explicitToJson: true)
class TaggingStatus {
  TaggingState status;
  String path;
  String? message;
  double? accuracy;
  bool? usedShazam;

  TaggingStatus(this.status, this.path, {this.message, this.accuracy, this.usedShazam});

  factory TaggingStatus.fromJson(Map<String, dynamic> json) => _$TaggingStatusFromJson(json);
  Map<String, dynamic> toJson() => _$TaggingStatusToJson(this);
}

enum TaggingState {
  @JsonValue("ok") ok,
  @JsonValue("error") error,
  @JsonValue("skipped") skipped
}


@JsonSerializable(explicitToJson: true)
class AutoTaggerPlatform {
  String id;
  bool builtIn;
  PlatformInfo platform;
  bool requiresAuth;
  List<String> supportedTags;

  @JsonKey(fromJson: AutoTaggerPlatform.decodeBase64Icon)
  Uint8List icon;

  AutoTaggerPlatform(this.id, this.builtIn, this.icon, this.platform, this.requiresAuth, this.supportedTags);

  /// Decode icon from base64
  static Uint8List decodeBase64Icon(String icon) {
    return base64Decode(icon.split(",").last);
  }


  factory AutoTaggerPlatform.fromJson(Map<String, dynamic> json) => _$AutoTaggerPlatformFromJson(json);
  Map<String, dynamic> toJson() => _$AutoTaggerPlatformToJson(this);
}

@JsonSerializable(explicitToJson: true)
class PlatformInfo {
  String id;
  String name;
  String description;
  String version;
  int maxThreads;
  List<String> supportedTags;
  bool requiresAuth;
  PlatformCustomOptions customOptions;

  PlatformInfo(this.id, this.name, this.description, this.version, this.maxThreads,
      this.supportedTags, this.requiresAuth, this.customOptions);

  factory PlatformInfo.fromJson(Map<String, dynamic> json) => _$PlatformInfoFromJson(json);
  Map<String, dynamic> toJson() => _$PlatformInfoToJson(this);
}

@JsonSerializable(explicitToJson: true)
class PlatformCustomOptions {
  List<PlatformCustomOption> options;

  PlatformCustomOptions(this.options);

  factory PlatformCustomOptions.fromJson(Map<String, dynamic> json) => _$PlatformCustomOptionsFromJson(json);
  Map<String, dynamic> toJson() => _$PlatformCustomOptionsToJson(this);
}

@JsonSerializable(explicitToJson: true)
class PlatformCustomOption {
  String id;
  String label;
  String? tooltip;
  PlatformCustomOptionValue value;

  PlatformCustomOption(this.id, this.label, this.value, {this.tooltip});

  factory PlatformCustomOption.fromJson(Map<String, dynamic> json) => _$PlatformCustomOptionFromJson(json);
  Map<String, dynamic> toJson() => _$PlatformCustomOptionToJson(this);
}

@JsonSerializable(explicitToJson: true)
class PlatformCustomOptionValue {
  PlatformCustomOptionValueType type;
  dynamic value;
  int? min;
  int? max;
  int? step;
  bool? hidden;
  List<String>? values;

  PlatformCustomOptionValue(this.type, this.value, {this.min, this.max, this.step, this.hidden, this.values});

  factory PlatformCustomOptionValue.fromJson(Map<String, dynamic> json) => _$PlatformCustomOptionValueFromJson(json);
  Map<String, dynamic> toJson() => _$PlatformCustomOptionValueToJson(this);
}

enum PlatformCustomOptionValueType {
  @JsonValue("boolean") boolean,
  @JsonValue("number") number,
  @JsonValue("string") string,
  @JsonValue("tag") tag,
  @JsonValue("option") option,
}