import 'package:flutter/widgets.dart';
import 'generated/frb_generated.dart';
import '../co_flutter.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';

Future<CoContext> openCoContext(CoSettings settings) async {
  await CoKit.init(externalLibrary: ExternalLibrary.open("libco_uniffi.dylib"));
  return await CoContext.open(settings: settings);
}

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

class CoPrivateIdentityProvider extends InheritedWidget {
  final CoPrivateIdentity identity;

  const CoPrivateIdentityProvider({
    super.key,
    required this.identity,
    required super.child,
  });

  static CoPrivateIdentity of(BuildContext context) {
    final ctx =
        context.dependOnInheritedWidgetOfExactType<CoPrivateIdentityProvider>();
    assert(ctx != null, 'CoPrivateIdentityProvider not found in widget tree');
    return ctx!.identity;
  }

  @override
  bool updateShouldNotify(CoPrivateIdentityProvider oldWidget) {
    return false;
  }
}
