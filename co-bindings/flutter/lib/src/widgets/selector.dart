import 'package:co_flutter/src/widgets/subscription.dart';
import 'package:flutter/widgets.dart';
import '../../co_flutter.dart';

class CoSelector<T, D> extends StatefulWidget {
  CoSelector({
    super.key,
    required this.select,
    required this.builder,
    required this.co,
    required this.deps,
    this.loading,
    this.errorBuilder,
    this.equalsDeps,
    this.equalsValue,
    this.initial,
  }) : notifier = CoSubscriptionNotifier(co.subscribe());

  /// CO
  final Co co;
  final CoSubscriptionNotifier notifier;

  /// Memoized dependencies (like React deps array)
  final D deps;

  /// Async selector
  final Future<T> Function(BlockStorage storage, CoState state, D deps) select;

  /// Render function
  final Widget Function(BuildContext context, T value) builder;

  /// Loading Render function
  final Widget? loading;
  final Widget Function(BuildContext context, Object error)? errorBuilder;

  /// Optional custom equality
  final bool Function(D a, D b)? equalsDeps;
  final bool Function(T a, T b)? equalsValue;

  final T? initial;

  @override
  State<CoSelector<T, D>> createState() => _CoSelectorState<T, D>();
}

class _CoSelectorState<T, D> extends State<CoSelector<T, D>> {
  T? _value;
  Object? _error;
  bool _loading = false;

  CoState? _lastState;
  D? _lastDeps;
  int _runId = 0;

  @override
  void initState() {
    super.initState();
    _value = widget.initial;
    widget.notifier.addListener(_invalidate);
    _maybeRun(force: true);
  }

  @override
  void didUpdateWidget(covariant CoSelector<T, D> oldWidget) {
    super.didUpdateWidget(oldWidget);

    if (oldWidget.notifier != widget.notifier) {
      oldWidget.notifier.removeListener(_invalidate);
      widget.notifier.addListener(_invalidate);
      _maybeRun(force: true);
    } else {
      _maybeRun();
    }
  }

  void _invalidate() {
    if (_stateChanged(widget.notifier.value)) {
      _lastState = widget.notifier.value;
      _maybeRun(force: true);
    }
  }

  bool _depsChanged(D next) {
    final prevDeps = _lastDeps;
    if (prevDeps == null) return true;
    return !(widget.equalsDeps?.call(prevDeps, next) ?? prevDeps == next);
  }

  bool _stateChanged(CoState? next) {
    return _lastState != next;
  }

  Future<void> _maybeRun({bool force = false}) async {
    final nextDeps = widget.deps;
    if (!force && !_depsChanged(nextDeps)) return;
    if (_lastState == null) return;

    _lastDeps = nextDeps;
    final myRun = ++_runId;

    setState(() {
      _loading = true;
      _error = null;
    });

    try {
      final next =
          await widget.select(await widget.co.storage(), _lastState!, nextDeps);
      if (!mounted || myRun != _runId) return;

      final sameValue = _value != null &&
          (widget.equalsValue?.call(_value as T, next) ?? _value == next);

      setState(() {
        _loading = false;
        if (!sameValue) _value = next;
      });
    } catch (e) {
      if (!mounted || myRun != _runId) return;
      setState(() {
        _loading = false;
        _error = e;
      });
    }
  }

  @override
  void dispose() {
    widget.notifier.removeListener(_invalidate);
    widget.notifier.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    if (_error != null) {
      return widget.errorBuilder?.call(context, _error!) ??
          Text('Error: $_error');
    }

    if (_loading && _value == null) {
      return widget.loading ?? const SizedBox.shrink();
    }

    if (_value == null) {
      return widget.loading ?? const SizedBox.shrink();
    }

    return widget.builder(context, _value as T);
  }
}
