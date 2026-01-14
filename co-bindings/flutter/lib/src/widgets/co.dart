import 'package:flutter/material.dart';
import '../../co_flutter.dart';

class CoProvider extends StatefulWidget {
  const CoProvider({
    super.key,
    required this.id,
    required this.child,
    this.loading,
    this.errorBuilder,
    this.onHandleReady,
  });

  final String id;
  final Widget child;
  final Widget? loading;
  final Widget Function(BuildContext context, Object error)? errorBuilder;
  final void Function(Co co)? onHandleReady;

  @override
  State<CoProvider> createState() => _CoProviderState();
}

class CoScope extends InheritedWidget {
  final Co co;

  const CoScope({
    super.key,
    required this.co,
    required super.child,
  });

  static Co of(BuildContext context) {
    final ctx = context.dependOnInheritedWidgetOfExactType<CoScope>();
    assert(ctx != null, 'CoScope not found in widget tree');
    return ctx!.co;
  }

  @override
  bool updateShouldNotify(CoScope oldWidget) {
    return false;
  }
}

class _CoProviderState extends State<CoProvider> {
  Co? _co;
  Object? _error;
  int _openToken = 0; // prevents race conditions

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _ensureHandle();
  }

  @override
  void didUpdateWidget(covariant CoProvider oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.id != widget.id) {
      _resetAndReopen();
    }
  }

  Future<void> _resetAndReopen() async {
    final old = _co;
    _co = null;
    _error = null;
    setState(() {});
    if (old != null) {
      try {
        old.dispose();
      } catch (_) {}
    }
    _ensureHandle();
  }

  void _ensureHandle() {
    if (_co != null || _error != null) return;

    final ctx = CoContextScope.of(context);
    final token = ++_openToken;

    ctx.openCo(id: widget.id).then((h) {
      if (!mounted || token != _openToken) {
        h.dispose();
        return;
      }
      setState(() {
        _co = h;
        _error = null;
      });
      widget.onHandleReady?.call(h);
    }).catchError((e) {
      if (!mounted || token != _openToken) return;
      setState(() => _error = e);
    });
  }

  @override
  void dispose() {
    final h = _co;
    _openToken++;
    _co = null;
    if (h != null) {
      h.dispose();
    }
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    if (_error != null) {
      final id = widget.id;
      return widget.errorBuilder?.call(context, _error!) ??
          Center(
            child: Text(
              'Failed to open CO: $id $_error',
              textAlign: TextAlign.center,
            ),
          );
    }

    final h = _co;
    if (h == null) {
      return widget.loading ?? const Center(child: CircularProgressIndicator());
    }

    return CoScope(
      co: h,
      child: widget.child,
    );
  }
}
