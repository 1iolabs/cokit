import 'package:flutter/widgets.dart';
import '../../co_flutter.dart';

class CoPrivateIdentityScope extends InheritedWidget {
  final CoPrivateIdentity identity;

  const CoPrivateIdentityScope({
    super.key,
    required this.identity,
    required super.child,
  });

  static CoPrivateIdentity of(BuildContext context) {
    final ctx =
        context.dependOnInheritedWidgetOfExactType<CoPrivateIdentityScope>();
    assert(ctx != null, 'CoPrivateIdentityScope not found in widget tree');
    return ctx!.identity;
  }

  @override
  bool updateShouldNotify(CoPrivateIdentityScope oldWidget) {
    return false;
  }
}
