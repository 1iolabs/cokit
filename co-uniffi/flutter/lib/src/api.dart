import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';
import 'generated/frb_generated.dart';
import '../co_flutter.dart';

Future<CoContext> openCoContext(CoSettings settings) async {
  await CoKit.init(externalLibrary: ExternalLibrary.open("libco_uniffi.dylib"));
  return await CoContext.open(settings: settings);
}

extension DagCborBlockStorage on BlockStorage {
  Future<T> getDagCbor<T>(Cid cid, {DagCborCodec<T>? codec}) async {
    final block = await getBlock(cid: cid);
    return DagCbor.decodeCodec(codec, block.data);
  }

  Future<Cid> setDagCbor<T>(T value, {DagCborCodec<T>? codec}) async {
    final data = DagCbor.encodeCodec(codec, value);
    final block = await Block.newData(codec: BigInt.from(0x71), data: data);
    return await setBlock(block: block);
  }
}
