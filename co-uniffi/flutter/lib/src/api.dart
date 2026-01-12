import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';
import 'generated/frb_generated.dart';
import '../co_flutter.dart';

Future<CoContext> openCoContext(CoSettings settings) async {
  await CoKit.init(externalLibrary: ExternalLibrary.open("libco_uniffi.dylib"));
  return await CoContext.open(settings: settings);
}
