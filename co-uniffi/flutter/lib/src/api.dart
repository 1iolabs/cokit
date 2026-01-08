import 'package:flutter/widgets.dart';
import 'generated/frb_generated.dart';
import '../co_flutter.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';

class CoContextProvider extends InheritedWidget {
  final CoContext context;

  const CoContextProvider({
    super.key,
    required this.context,
    required super.child,
  });

  static CoContext of(BuildContext context) {
    final ctx = context.dependOnInheritedWidgetOfExactType<CoContextProvider>();
    assert(ctx != null, 'CoContextProvider not found in widget tree');
    return ctx!.context;
  }

  @override
  bool updateShouldNotify(CoContextProvider oldWidget) {
    return false;
  }
}

Future<CoContext> openCoContext(CoSettings settings) async {
  await CoKit.init(externalLibrary: ExternalLibrary.open("libco_uniffi.dylib"));
  return await CoContext.open(settings: settings);
}
