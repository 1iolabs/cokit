import 'package:co_flutter/src/dag_cbor.dart';
import 'package:co_flutter/src/generated/types/storage.dart';
import '../generated/types/cid.dart';
import '../generated/types/co_map.dart' as bridge;

class CoMap<K, V> implements DagCborCodecProvider {
  bridge.CoMap _inner;
  final DagCborCodec<K>? _keyCodec;
  final DagCborCodec<V>? _codec;

  CoMap(Cid? root, {DagCborCodec<K>? keyCodec, DagCborCodec<V>? codec})
      : _inner = bridge.CoMap(root: root),
        _keyCodec = keyCodec,
        _codec = codec;
  Cid? get cid => _inner.root;

  @override
  DagCborCodec<dynamic> get dagCborCodec => _CoMapDagCborCodec();

  Future<bool> isEmpty() async {
    return await _inner.isEmpty();
  }

  Future<V?> getValue(BlockStorage storage, K key) async {
    final value = await _inner.getValue(
        storage: storage, key: DagCbor.encodeCodec(_keyCodec, key));
    if (value == null) {
      return null;
    }
    return DagCbor.decodeCodec(_codec, value);
  }

  Future<bool> contains(BlockStorage storage, K key) async {
    return await _inner.contains(
        storage: storage, key: DagCbor.encodeCodec(_keyCodec, key));
  }

  Future<void> insert(BlockStorage storage, K key, V value) async {
    _inner = await _inner.insert(
        storage: storage,
        key: DagCbor.encodeCodec(_keyCodec, key),
        value: DagCbor.encodeCodec(_codec, value));
  }

  Future<List<(K, V)>> entries(BlockStorage storage,
      {BigInt? skip, BigInt? limit}) async {
    final result =
        await _inner.entries(storage: storage, skip: skip, limit: limit);
    return result.map((item) {
      final (key, value) = item;
      return (
        DagCbor.decodeCodec(_keyCodec, key),
        DagCbor.decodeCodec(_codec, value)
      );
    }).toList();
  }

  Stream<(K, V)> stream(BlockStorage storage) {
    final result = _inner.stream(storage: storage);
    return result.takeWhile((item) => item != null).map((item) {
      final (key, value) = item!;
      return (
        DagCbor.decodeCodec(_keyCodec, key),
        DagCbor.decodeCodec(_codec, value)
      );
    });
  }
}

class _CoMapDagCborCodec<K, V> implements DagCborCodec<CoMap<K, V>> {
  @override
  CoMap<K, V> fromDagCborValue(value) {
    return CoMap(value as Cid);
  }

  @override
  toDagCborValue(CoMap<K, V> value) {
    return value.cid;
  }
}
