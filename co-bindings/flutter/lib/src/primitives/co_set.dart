import 'package:co_flutter/src/dag_cbor.dart';
import 'package:co_flutter/src/generated/types/storage.dart';
import '../generated/types/cid.dart';
import '../generated/types/co_set.dart' as bridge;

class CoSet<V> implements DagCborCodecProvider {
  bridge.CoSet _inner;
  final DagCborCodec<V>? _codec;

  CoSet(Cid? root, {DagCborCodec<V>? codec})
      : _inner = bridge.CoSet(root: root),
        _codec = codec;
  Cid? get cid => _inner.root;

  @override
  DagCborCodec<dynamic> get dagCborCodec => _CoSetDagCborCodec();

  Future<bool> isEmpty() async {
    return await _inner.isEmpty();
  }

  Future<bool> contains(BlockStorage storage, V value) async {
    return await _inner.contains(
        storage: storage, key: DagCbor.encodeCodec(_codec, value));
  }

  Future<void> insert(BlockStorage storage, V value) async {
    _inner = await _inner.insert(
        storage: storage, value: DagCbor.encodeCodec(_codec, value));
  }

  Future<List<V>> entries(BlockStorage storage,
      {BigInt? skip, BigInt? limit}) async {
    final result =
        await _inner.entries(storage: storage, skip: skip, limit: limit);
    return result.map((value) {
      return (DagCbor.decodeCodec(_codec, value));
    }).toList();
  }

  Stream<V> stream(BlockStorage storage) {
    final result = _inner.stream(storage: storage);
    return result.takeWhile((item) => item != null).map((value) {
      return (DagCbor.decodeCodec(_codec, value!));
    });
  }
}

class _CoSetDagCborCodec<V> implements DagCborCodec<CoSet<V>> {
  @override
  CoSet<V> fromDagCborValue(value) {
    return CoSet(value as Cid);
  }

  @override
  toDagCborValue(CoSet<V> value) {
    return value.cid;
  }
}
