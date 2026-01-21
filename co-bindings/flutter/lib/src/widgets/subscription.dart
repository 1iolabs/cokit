import 'dart:async';

import 'package:flutter/material.dart';
import '../../co_flutter.dart';

class CoSubscriptionNotifier extends ValueNotifier<CoState?> {
  final CoSubscription _subscription;
  StreamSubscription<CoState>? _streamSubscription;

  CoSubscriptionNotifier(this._subscription) : super(null) {
    _streamSubscription = _subscription.stream().listen(_onChange);
  }

  void _onChange(CoState next) {
    value = next;
  }

  @override
  void dispose() {
    _streamSubscription?.cancel().then((_) {
      _subscription.close();
    });
    super.dispose();
  }
}
