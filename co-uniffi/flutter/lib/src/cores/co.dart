import 'dart:typed_data';
import '../../co_flutter.dart';

/// See: ../../../../../cores/co/src/lib.rs
class CoreCo {
  final String /* CoId */ id;
  final List<dynamic>? /* Tags */ tags;
  final String /* String */ name;
  final Cid /* Cid */ binary;
  final dynamic /* CoMap<Did, Participant> */ participants;
  final Map<String, Core> /* BTreeMap<String, Core> */ cores;
  final Map<String, Map<String, dynamic>>? /* BTreeMap<String, Guard> */ guards;
  final dynamic /* Option<Vec<Key>> */ keys;
  final dynamic /* CoSet<Network> */ network;

  CoreCo({
    required this.id,
    required this.tags,
    required this.name,
    required this.binary,
    required this.participants,
    required this.cores,
    required this.guards,
    required this.keys,
    required this.network,
  });

  static CoreCo fromDagCbor(Uint8List bytes) {
    final data = DagCbor.decode(bytes);
    return CoreCo(
      id: data["id"],
      tags: data["t"],
      name: data["n"],
      binary: data["b"],
      participants: data["p"],
      cores: (data["c"] as Map<dynamic, dynamic>).map((core_name, core) => MapEntry(core_name as String, Core.fromDynamic(core))),
      guards: data["g"],
      keys: data["k"],
      network: data["s"],
    );
  }
}

class Core {
  final Cid /* Cid */ binary;
  final dynamic /* Tags */ tags;
  final Cid? /* Option<Cid> */ state;

  Core({required this.binary, required this.tags, required this.state});

  static Core fromDagCbor(Uint8List bytes) {
    return Core.fromDynamic(DagCbor.decode(bytes));
  }

  static Core fromDynamic(dynamic data) {
    return Core(
      binary: data["binary"],
      tags: data["tags"],
      state: data["state"],
    );
  }
}
