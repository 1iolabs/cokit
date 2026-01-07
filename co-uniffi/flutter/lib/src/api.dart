import 'package:flutter/widgets.dart';
import 'generated/co_uniffi.dart';

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
  return await co_context_open(settings);
}

CoSettings createCoSettings(
  String identifier, {
  String? path,
  CoNetworkSettings? network_settings,
  bool? network,
  bool? no_keychain,
  bool? no_log,
  CoLogLevel? log_level,
  bool? no_default_features,
  List<String>? feature,
}) {
  return co_settings_new(identifier);
}
