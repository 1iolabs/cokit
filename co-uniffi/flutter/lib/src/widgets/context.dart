import 'package:flutter/widgets.dart';
import '../../co_flutter.dart';

class CoContextScope extends InheritedWidget {
  final CoContext context;

  const CoContextScope({
    super.key,
    required this.context,
    required super.child,
  });

  static CoContext of(BuildContext context) {
    final ctx = context.dependOnInheritedWidgetOfExactType<CoContextScope>();
    assert(ctx != null, 'CoContextScope not found in widget tree');
    return ctx!.context;
  }

  @override
  bool updateShouldNotify(CoContextScope oldWidget) {
    return false;
  }
}
