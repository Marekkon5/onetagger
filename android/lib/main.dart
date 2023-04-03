import 'package:dio/dio.dart';
import 'package:flutter/material.dart';
import 'package:onetagger_android/api.dart';
import 'package:onetagger_android/autotagger.dart';
import 'package:onetagger_android/settings.dart';
import 'package:path_provider/path_provider.dart';
import 'package:flutter_native_splash/flutter_native_splash.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:version/version.dart';

import 'dart:convert';
import 'dart:developer';


void main() {
  // Keep splash screen
  WidgetsBinding widgetsBinding = WidgetsFlutterBinding.ensureInitialized();
  FlutterNativeSplash.preserve(widgetsBinding: widgetsBinding);

  runApp(const App());
}

class App extends StatefulWidget {
  const App({super.key});

  @override
  State<App> createState() => _AppState();
}

class _AppState extends State<App> {

  ThemeData themeData = ThemeData(
    colorScheme: ColorScheme.fromSeed(seedColor: const Color(0xaa00d2bf), brightness: Brightness.dark),
    fontFamily: 'Dosis',
    useMaterial3: true,
    brightness: Brightness.dark
  );

  // Load 1T ffi
  void load() async {
    // Initialize 1T
    var dataDir = await getApplicationDocumentsDirectory();
    await onetaggerAndroid.init(dataDir: dataDir.absolute.path);

    // Load settings
    settings = await Settings.load();
    autoTaggerConfig = settings.autoTaggerConfig;
    themeData = await settings.themeData();

    // Load custom configs
    autoTaggerConfig.applyCustom(jsonDecode(await onetaggerAndroid.customDefault()));
    autoTaggerPlatforms = jsonDecode(await onetaggerAndroid.platforms())
        .map<AutoTaggerPlatform>((p) => AutoTaggerPlatform.fromJson(p))
        .toList();

    // Update theme
    setState(() {});

    // Hide splash
    FlutterNativeSplash.remove();
  }

  @override
  void initState() {
    load();
    super.initState();
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      theme: themeData,
      home: const HomeScreen(),
    );
  }
}


class HomeScreen extends StatelessWidget {
  const HomeScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Container(
        decoration: const BoxDecoration(
          image: DecorationImage(
            image: AssetImage("assets/bg.jpg"),
            opacity: 0.5,
            fit: BoxFit.cover
          )
        ),
        child: ListView(
          children: [
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 32.0, vertical: 16.0),
              child: Image.asset("assets/banner.png"),
            ),
      
            const VersionWidget(),
      
            const Padding(
              padding: EdgeInsets.symmetric(vertical: 16.0, horizontal: 8.0),
              child: Text(
                "The Android version of OneTagger only supports AutoTagger, which allows you to quickly and automatically fetch correct metadata for all Your audio files.",
                style: TextStyle(
                  fontSize: 16.0
                )
              ),
            ),
      
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16.0),
              child: ElevatedButton(
                child: const Text('AutoTagger'),
                onPressed: () {
                  Navigator.of(context).push(MaterialPageRoute(builder: (context) => const AutoTaggerScreen()));
                },
              ),
            ),
      
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16.0),
              child: ElevatedButton(
                child: const Text('Settings'),
                onPressed: () {
                  Navigator.of(context).push(MaterialPageRoute(builder: (context) => const SettingsScreen()));
                },
              ),
            ),

            Container(height: 32.0),
            const UpdaterWidget(),
          ],
        ),
      )
    );
  }
}

class VersionWidget extends StatefulWidget {
  const VersionWidget({Key? key}) : super(key: key);

  @override
  State<VersionWidget> createState() => _VersionWidgetState();
}

class _VersionWidgetState extends State<VersionWidget> {
  String version = '';

  @override
  void initState() {
    onetaggerAndroid.version().then((v) => setState(() => version = '${v.version} (Commit: ${v.commit})'));
    super.initState();
  }

  @override
  Widget build(BuildContext context) {
    return Text('v$version', textAlign: TextAlign.center, style: const TextStyle(fontFamily: 'monospace'));
  }
}


class UpdaterWidget extends StatelessWidget {
  const UpdaterWidget({super.key});

  /// Check for updates
  Future<String?> _checkUpdate() async {
    var version = (await onetaggerAndroid.version()).version;
    // Get latest version number
    var dio = Dio();
    var response = await dio.get('https://1t.marekkon5.workers.dev/latest', options: Options(
      responseType: ResponseType.json
    ));
    var latest = response.data['version'];
    // Compare
    var currentVersion = Version.parse(version);
    var latestVersion = Version.parse(latest);

    if (latestVersion > currentVersion) {
      return response.data['url'];
    }
    return null;
  }

  @override
  Widget build(BuildContext context) {
    return FutureBuilder(
      future: _checkUpdate(),
      builder: (context, snapshot) {
        
        // Show next version link
        if (snapshot.data != null) {
          return Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(
                "New version available!",
                textAlign: TextAlign.center,
                style: TextStyle(
                  fontSize: 16.0,
                  fontWeight: FontWeight.bold,
                  color: Theme.of(context).colorScheme.primary
                )
              ),
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 16.0, vertical: 8.0),
                child: ElevatedButton(
                  child: const Text("Open download page"),
                  onPressed: () {
                    launchUrl(Uri.parse(snapshot.data!), mode: LaunchMode.externalApplication);
                  },
                ),
              )
            ],
          );
        }

        // Error info
        if (snapshot.hasError) {
          log('Failed fetching updates: ${snapshot.error}');
        }

        return Container();
      }
    );
  }
}


