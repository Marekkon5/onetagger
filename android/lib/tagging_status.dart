import 'package:flutter/material.dart';
import 'package:material_design_icons_flutter/material_design_icons_flutter.dart';
import 'package:onetagger_android/api.dart';
import 'package:onetagger_android/log.dart';

import 'dart:async';
import 'dart:convert';

import 'package:onetagger_android/main.dart';

// Status counter
class TaggerStatus {
  List<TaggingStatusWrap> statuses = [];
  DateTime started = DateTime.now();
  double progress = 0.0;
  bool done = false;
  DateTime? doneTime;

  TaggerStatus() {
    started = DateTime.now();
  }

  // Add new status
  addStatus(TaggingStatusWrap statusWrap) {
    var i = statuses.indexWhere((s) => s.status.path == statusWrap.status.path);
    if (i != -1) {
      statuses.removeAt(i);
    }
    statuses.insert(0, statusWrap);
    progress = statusWrap.progress;
  }

  // Count by state
  int countState(TaggingState state) {
    return statuses.fold(0, (a, v) => (v.status.status == state) ? a + 1 : a);
  }

  // Formatted elapsed time string
  String elapsedFormatted() {
    var d = DateTime.now().difference(started);
    if (doneTime != null) d = doneTime!.difference(started);
    return "${d.inMinutes.remainder(60).toString()}:${d.inSeconds.remainder(60).toString().padLeft(2, '0')}";
  }

}

class TaggingStatusScreen extends StatefulWidget {
  const TaggingStatusScreen({Key? key}) : super(key: key);

  @override
  State<TaggingStatusScreen> createState() => _TaggingStatusScreenState();
}

class _TaggingStatusScreenState extends State<TaggingStatusScreen> {

  bool cancelTimer = false;
  TaggerStatus status = TaggerStatus();

  @override
  void initState() {
    startTimer();
    super.initState();
  }

  @override
  void dispose() {
    cancelTimer = true;
    super.dispose();
  }

  void startTimer() {
    cancelTimer = false;
    Timer.periodic(const Duration(milliseconds: 400), (timer) async {
      // Stop
      if (cancelTimer) {
        timer.cancel();
        return;
      }

      // Get statuses
      var statuses = await onetaggerAndroid.getStatuses();
      for (String statusJson in statuses) {
        var statusWrap = TaggingStatusWrap.fromJson(jsonDecode(statusJson));
        status.addStatus(statusWrap);
      }

      // Check if is done
      if (!status.done && await onetaggerAndroid.isDone()) {
        status.done = true;
        status.doneTime = DateTime.now();
        status.progress = 1.0;
        timer.cancel();

        ScaffoldMessenger.of(context).showSnackBar(const SnackBar(
          content: Text('Tagging finished!'),
        ));
      }

      setState(() {});
    });
  }


  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text(
          'Tagging Status',
          style: TextStyle(
            fontWeight: FontWeight.bold
          ),
        ),
        centerTitle: true,
        actions: [
          IconButton(
            icon: const Icon(Icons.text_snippet),
            onPressed: () {
              Navigator.of(context).push(MaterialPageRoute(builder: (context) => const LogScreen()));
            },
          )
        ],
      ),
      floatingActionButton: (!status.done) ? null : FloatingActionButton(
        child: const Icon(Icons.close),
        onPressed: () {
          Navigator.of(context).pushReplacement(MaterialPageRoute(builder: (context) => const HomeScreen()));
        },
      ),
      body: Column(
        mainAxisSize: MainAxisSize.max,
        children: [
          // Items info
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceAround,
            children: [
              StatusCard(status.countState(TaggingState.ok).toString(), Icons.done, Colors.green),
              StatusCard(status.countState(TaggingState.error).toString(), Icons.error, Colors.yellow),
              StatusCard(status.countState(TaggingState.skipped).toString(), MdiIcons.debugStepOver, Colors.red),
              StatusCard('${(status.progress * 100.0).round()}%', Icons.timelapse, Colors.blue),
              StatusCard(status.elapsedFormatted(), Icons.timer, Colors.purple),
            ],
          ),
          Container(height: 16.0),

          // Show statuses
          Expanded(
            child: SingleChildScrollView(
              child: Column(
                children: List.generate(status.statuses.length, (i) =>
                  TaggingStatusWidget(status.statuses[i])
                ),
              ),
            ),
          ),

          // Progress bar
          Container(height: 16.0),
          LinearProgressIndicator(
            value: status.progress,
          )
        ],
      ),
    );
  }
}

class StatusCard extends StatelessWidget {
  final String text;
  final IconData icon;
  final Color color;

  const StatusCard(this.text, this.icon, this.color, {Key? key}) : super(key: key);

  @override
  Widget build(BuildContext context) {
    return Card(
      color: color.withAlpha(48),
      child: Padding(
        padding: const EdgeInsets.all(8.0),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(icon, size: 12.0),
            Container(width: 8.0),
            Text(
              text,
              style: const TextStyle(
                fontFamily: 'monospace',
                fontSize: 12.0
              ),
            )
          ],
        ),
      )
    );
  }
}

class TaggingStatusWidget extends StatelessWidget {
  final TaggingStatusWrap status;

  const TaggingStatusWidget(this.status, {Key? key}) : super(key: key);

  // Get leading icon
  Widget leading() {
    switch (status.status.status) {
      case TaggingState.ok:
        return const Icon(Icons.done, color: Colors.green);
      case TaggingState.error:
        return const Icon(Icons.error, color: Colors.red);
      case TaggingState.skipped:
        return const Icon(MdiIcons.debugStepOver, color: Colors.yellow);
    }
  }

  @override
  Widget build(BuildContext context) {
    return ListTile(
      leading: leading(),
      title: Text(
        status.platform.toUpperCase(),
        style: const TextStyle(
          fontWeight: FontWeight.bold,
        ),
      ),
      subtitle: Text(
        status.status.path,
        maxLines: 2,
        style: const TextStyle(
          fontFamily: 'monospace',
          fontSize: 11.0
        ),
      ),
    );
  }
}
