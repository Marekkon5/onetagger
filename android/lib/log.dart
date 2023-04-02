import 'dart:async';

import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:onetagger_android/api.dart';
import 'package:onetagger_android/api_generated.dart';

class LogScreen extends StatefulWidget {
  const LogScreen({Key? key}) : super(key: key);

  @override
  State<LogScreen> createState() => _LogScreenState();
}

class _LogScreenState extends State<LogScreen> {

  ScrollController scrollController = ScrollController();
  List<LogMessage> logs = [];
  bool disposed = false;
  bool isAtBottom = false;
  bool autoScroll = true;

  // Load logs from FFI
  void loadLogs() async {
    logs = await onetaggerAndroid.logs();
    setState(() {});
    if (autoScroll) {
      scrollController.jumpTo(scrollController.position.maxScrollExtent);
    }
  }

  @override
  void initState() {
    scrollController.addListener(onScrollEvent);
    disposed = false;
    loadLogs();

    // Start log update timer
    Timer.periodic(const Duration(milliseconds: 500), (timer) {
      if (disposed) {
        timer.cancel();
        return;
      }
      loadLogs();
    });

    super.initState();
  }

  @override
  void dispose() {
    scrollController.removeListener(onScrollEvent);
    disposed = true;
    super.dispose();
  }

  /// Handle scroll events
  void onScrollEvent() {
    setState(() {
      isAtBottom = scrollController.offset >= scrollController.position.maxScrollExtent;
      if (!isAtBottom) {
        autoScroll = false;
      }
    });

  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text(
          'Logs',
          style: TextStyle(
            fontWeight: FontWeight.bold
          ),
        ),
        centerTitle: true,
      ),
      floatingActionButton: (isAtBottom) ? null : FloatingActionButton(
        child: const Icon(Icons.arrow_downward),
        onPressed: () {
          scrollController.jumpTo(
            scrollController.position.maxScrollExtent + 100.0,
          );
          isAtBottom = true;
          autoScroll = true;
        },
      ),
      body: ListView(
        controller: scrollController,
        children: List.generate(logs.length, (i) => Padding(
          padding: const EdgeInsets.symmetric(vertical: 8.0, horizontal: 4.0),
          child: LogMessageWidget(logs[i]),
        )),
      )
    );
  }
}

class LogMessageWidget extends StatelessWidget {
  final LogMessage message;
  final DateFormat dateFormat = DateFormat('yyyy-MM-dd hh:mm:ss');
  
  LogMessageWidget(this.message, {Key? key}) : super(key: key);
  
  String levelString(LogLevel level) {
    switch (level) {
      case LogLevel.Trace:
        return 'TRACE';
      case LogLevel.Debug:
        return 'DEBUG';
      case LogLevel.Warn:
        return 'WARN';
      case LogLevel.Info:
        return 'INFO';
      case LogLevel.Error:
        return 'ERROR';
    }
  }
  
  Color levelColor(LogLevel level) {
    switch (level) {
      case LogLevel.Trace:
        return Colors.grey;
      case LogLevel.Debug:
        return Colors.blue;
      case LogLevel.Warn:
        return Colors.yellow;
      case LogLevel.Info:
        return Colors.green;
      case LogLevel.Error:
        return Colors.red;
    }
  }
  
  @override
  Widget build(BuildContext context) {
    return RichText(
      text: TextSpan(
        text: '',
        children: [
          TextSpan(
            text: levelString(message.level),
            style: TextStyle(
              color: levelColor(message.level),
              fontWeight: FontWeight.bold,
              fontFamily: 'monospace',
              fontSize: 12.0
            )
          ),
          const TextSpan(text: '  '),
          TextSpan(
            text: message.module,
            style: const TextStyle(
              fontWeight: FontWeight.bold,
              fontFamily: 'monospace',
                fontSize: 12.0
            )
          ),
          const TextSpan(text: '  '),
          TextSpan(
            text: dateFormat.format(DateTime.fromMillisecondsSinceEpoch(message.time * 1000).toLocal()),
            style: const TextStyle(
              fontWeight: FontWeight.bold,
              fontFamily: 'monospace',
              color: Colors.grey,
              fontSize: 12.0
            )
          ),
          const TextSpan(text: '\n'),
          TextSpan(text: message.message, style: const TextStyle(fontFamily: 'monospace'))
        ]
      )
    );
  }
}
