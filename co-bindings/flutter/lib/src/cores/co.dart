import '../../co_flutter.dart';

/// See: ../../../../../cores/co/src/lib.rs
class CoreCo implements DagCborCodecProvider {
  final String /* CoId */ id;
  final Tags /* Tags */ tags;
  final String /* String */ name;
  final Cid /* Cid */ binary;
  final CoMap<String, Participant> /* CoMap<Did, Participant> */ participants;
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

  static final codec = CoreCoDagCborCodec();

  @override
  DagCborCodec<dynamic> get dagCborCodec => codec;
}

class CoreCoDagCborCodec implements DagCborCodec<CoreCo> {
  @override
  CoreCo fromDagCborValue(data) {
    return CoreCo(
      id: data["id"],
      tags: Tags.codec.fromDagCborValue(data["t"]),
      name: data["n"],
      binary: data["b"],
      participants: CoMap(data["p"], codec: Participant.codec),
      cores: (data["c"] as Map<dynamic, dynamic>).map((core_name, core) =>
          MapEntry(core_name as String, Core.codec.fromDagCborValue(core))),
      guards: data["g"],
      keys: data["k"],
      network: data["s"],
    );
  }

  @override
  dynamic toDagCborValue(CoreCo value) {
    return {
      "id": value.id,
      "t": value.tags.toDagCborValue(),
      "n": value.name,
      "b": value.binary,
      "p": value.participants,
      "c": value.cores
          .map((core_name, core) => MapEntry(core_name, core.toDagCborValue())),
      "g": value.guards,
      "k": value.keys,
      "s": value.network,
    };
  }
}

class Core implements DagCborCodecProvider {
  final Cid /* Cid */ binary;
  final dynamic /* Tags */ tags;
  final Cid? /* Option<Cid> */ state;

  Core({required this.binary, required this.tags, required this.state});

  static final codec = CoreDagCborCodec();

  @override
  DagCborCodec<dynamic> get dagCborCodec => codec;
}

class CoreDagCborCodec implements DagCborCodec<Core> {
  @override
  Core fromDagCborValue(value) {
    return Core(
      binary: value["binary"],
      tags: value["tags"],
      state: value["state"],
    );
  }

  @override
  dynamic toDagCborValue(Core value) {
    return {
      "binary": value.binary,
      "tags": value.tags,
      "state": value.state,
    };
  }
}

class Participant implements DagCborCodecProvider {
  final String /* Did */ did;
  final ParticipantState /* ParticipantState */ state;
  final Tags /* Tags */ tags;

  Participant({required this.did, required this.state, required this.tags});

  static final codec = ParticipantDagCborCodec();

  @override
  DagCborCodec<dynamic> get dagCborCodec => codec;
}

class ParticipantDagCborCodec implements DagCborCodec<Participant> {
  @override
  Participant fromDagCborValue(value) {
    return Participant(
      did: value["did"],
      state: ParticipantState.codec.fromDagCborValue(value["state"]),
      tags: Tags.codec.fromDagCborValue(value["tags"]),
    );
  }

  @override
  dynamic toDagCborValue(Participant value) {
    return {
      "did": value.did,
      "state": value.state.toDagCborValue(),
      "tags": value.tags.toDagCborValue(),
    };
  }
}

enum ParticipantState implements DagCborCodecProvider {
  active(0),
  inactive(1),
  invite(2),
  pending(3);

  final int value;
  const ParticipantState(this.value);

  factory ParticipantState.fromInt(int value) {
    return ParticipantState.values.firstWhere(
      (e) => e.value == value,
      orElse: () =>
          throw ArgumentError('Invalid ParticipantState value: $value'),
    );
  }

  static final codec = ParticipantStateDagCborCodec();

  @override
  DagCborCodec<dynamic> get dagCborCodec => codec;
}

class ParticipantStateDagCborCodec implements DagCborCodec<ParticipantState> {
  @override
  fromDagCborValue(value) {
    return ParticipantState.fromInt(value);
  }

  @override
  toDagCborValue(ParticipantState value) {
    return value.value;
  }
}
