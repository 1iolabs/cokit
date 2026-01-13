import 'dart:convert';
import 'dart:typed_data';
import 'generated/types/cid.dart';

abstract interface class DagCborCodecProvider {
  DagCborCodec<dynamic> get dagCborCodec;
}

extension DagCborConvenience on DagCborCodecProvider {
  dynamic toDagCborValue() => dagCborCodec.toDagCborValue(this);
  Uint8List toDagCbor() => DagCbor.encodeCodec(dagCborCodec, this);
}

abstract interface class DagCborCodec<T> {
  T fromDagCborValue(dynamic value);
  dynamic toDagCborValue(T value);
}

class DagCbor {
  static Uint8List encode(dynamic value) {
    final w = _Writer();
    _encodeDag(value, w);
    return w.toBytes();
  }

  static dynamic decode(Uint8List bytes) {
    final r = _Reader(bytes);
    final v = _decodeDag(r);
    r.ensureDone();
    return v;
  }

  static Uint8List encodeCodec<T>(
    DagCborCodec<T>? codec,
    T value,
  ) {
    if (codec != null) {
      return encode(codec.toDagCborValue(value));
    }
    return encode(value);
  }

  static T decodeCodec<T>(
    DagCborCodec<T>? codec,
    Uint8List bytes,
  ) {
    if (codec != null) {
      return codec.fromDagCborValue(decode(bytes));
    }
    return decode(bytes);
  }

  static void _encodeDag(dynamic v, _Writer w) {
    if (v is DagCborCodecProvider) {
      v = v.toDagCborValue();
    }
    if (v == null) {
      w.writeSimple(22); // null
      return;
    }
    if (v is bool) {
      w.writeSimple(v ? 21 : 20);
      return;
    }
    if (v is int) {
      w.writeInt(v);
      return;
    }
    if (v is double) {
      // DAG-CBOR allows floats, but you may want to restrict NaN/Inf.
      if (v.isNaN || v.isInfinite) {
        throw FormatException('DAG-CBOR forbids NaN/Infinity.');
      }
      w.writeFloat64(v);
      return;
    }
    if (v is String) {
      w.writeText(v);
      return;
    }
    if (v is Uint8List) {
      w.writeBytes(v);
      return;
    }
    if (v is Cid) {
      w.writeTag(42);
      final payload = Uint8List(v.bytes.length + 1);
      payload[0] = 0x00;
      payload.setRange(1, payload.length, v.bytes);
      w.writeBytes(payload);
      return;
    }
    if (v is List) {
      w.writeArrayHeader(v.length);
      for (final e in v) {
        _encodeDag(e, w);
      }
      return;
    }
    if (v is Map) {
      // DAG-CBOR requires map keys to be strings.
      final entries = <MapEntry<String, dynamic>>[];
      v.forEach((k, val) {
        if (k is! String) {
          throw FormatException(
            'DAG-CBOR map keys must be strings. Got: ${k.runtimeType}',
          );
        }
        entries.add(MapEntry<String, dynamic>(k, val));
      });

      // Sort by bytewise ordering of the CBOR-encoded key.
      entries.sort((a, b) {
        final ak = _cborEncodedTextKey(a.key);
        final bk = _cborEncodedTextKey(b.key);
        return _lexCompare(ak, bk);
      });

      w.writeMapHeader(entries.length);
      for (final e in entries) {
        w.writeText(e.key);
        _encodeDag(e.value, w);
      }
      return;
    }

    throw FormatException('Unsupported type for DAG-CBOR: ${v.runtimeType}');
  }

  static dynamic _decodeDag(_Reader r) {
    final initial = r.peekByte();
    if (initial == null) throw FormatException('Unexpected EOF');

    final major = initial >> 5;
    final ai = initial & 0x1f;

    switch (major) {
      case 0: // unsigned int
        return r.readUnsigned();
      case 1: // negative int
        return r.readNegative();
      case 2: // bytes
        final b = r.readBytes();
        return b;
      case 3: // text
        return r.readText();
      case 4: // array
        final n = r.readLengthDefinite();
        final list = <dynamic>[];
        for (var i = 0; i < n; i++) {
          list.add(_decodeDag(r));
        }
        return list;
      case 5: // map
        final n = r.readLengthDefinite();
        final map = <String, dynamic>{};
        Uint8List? prevKeyEnc;
        for (var i = 0; i < n; i++) {
          // Keys must be text.
          final keyStart = r.pos;
          final key = _decodeDag(r);
          if (key is! String) {
            throw FormatException('DAG-CBOR map keys must be strings.');
          }
          // Verify canonical ordering if you want strictness:
          final keyEnc = r.bytes.sublist(keyStart, r.pos);
          if (prevKeyEnc != null && _lexCompare(prevKeyEnc, keyEnc) >= 0) {
            throw FormatException('Non-canonical map key order.');
          }
          prevKeyEnc = keyEnc;

          final val = _decodeDag(r);
          map[key] = val;
        }
        return map;
      case 6: // tag
        final tag = r.readTagNumber();
        if (tag == 42) {
          final payload = _decodeDag(r);
          if (payload is! Uint8List) {
            throw FormatException('CID tag 42 must wrap a byte string.');
          }
          if (payload.isEmpty || payload[0] != 0x00) {
            throw FormatException('CID byte string must start with 0x00.');
          }
          final cidBytes = Uint8List.fromList(payload.sublist(1));
          return Cid(bytes: cidBytes);
        } else {
          // For unknown tags: decode and return as a tagged structure.
          final inner = _decodeDag(r);
          return _Tagged(tag, inner);
        }
      case 7: // simple/float
        if (ai == 20) return r.readSimple(); // false
        if (ai == 21) return r.readSimple(); // true
        if (ai == 22) return r.readSimple(); // null
        if (ai == 27) return r.readFloat64();
        // You can add float16/float32 handling if needed.
        return r.readSimpleOrUnknown();
      default:
        throw FormatException('Unknown CBOR major type: $major');
    }
  }

  static Uint8List _cborEncodedTextKey(String s) {
    final w = _Writer();
    w.writeText(s);
    return w.toBytes();
  }

  static int _lexCompare(Uint8List a, Uint8List b) {
    final n = a.length < b.length ? a.length : b.length;
    for (var i = 0; i < n; i++) {
      final d = a[i] - b[i];
      if (d != 0) return d;
    }
    return a.length - b.length;
  }
}

class _Tagged {
  final int tag;
  final dynamic value;
  _Tagged(this.tag, this.value);

  @override
  String toString() => 'Tagged($tag, $value)';
}

/// CBOR writer (definite-length only).
class _Writer {
  final BytesBuilder _b = BytesBuilder(copy: false);

  Uint8List toBytes() => _b.toBytes();

  void writeByte(int v) => _b.addByte(v & 0xff);
  void writeAll(List<int> v) => _b.add(v);

  void writeTypeAndInt(int major, int n) {
    if (n < 0) throw ArgumentError('n must be >= 0');
    if (n <= 23) {
      writeByte((major << 5) | n);
    } else if (n <= 0xff) {
      writeByte((major << 5) | 24);
      writeByte(n);
    } else if (n <= 0xffff) {
      writeByte((major << 5) | 25);
      _writeU16(n);
    } else if (n <= 0xffffffff) {
      writeByte((major << 5) | 26);
      _writeU32(n);
    } else {
      writeByte((major << 5) | 27);
      _writeU64(n);
    }
  }

  void writeInt(int v) {
    // Restrict to signed 64-bit.
    // Dart int is arbitrary precision on VM, but in JS it’s 53-bit; treat carefully.
    if (v >= 0) {
      writeTypeAndInt(0, v);
    } else {
      final n = -1 - v;
      writeTypeAndInt(1, n);
    }
  }

  void writeBytes(Uint8List bytes) {
    writeTypeAndInt(2, bytes.length);
    writeAll(bytes);
  }

  void writeText(String s) {
    final data = utf8.encode(s);
    writeTypeAndInt(3, data.length);
    writeAll(data);
  }

  void writeArrayHeader(int n) => writeTypeAndInt(4, n);
  void writeMapHeader(int n) => writeTypeAndInt(5, n);

  void writeTag(int tag) => writeTypeAndInt(6, tag);

  void writeSimple(int simple) {
    // major 7
    if (simple <= 23) {
      writeByte((7 << 5) | simple);
    } else {
      writeByte((7 << 5) | 24);
      writeByte(simple);
    }
  }

  void writeFloat64(double v) {
    writeByte((7 << 5) | 27);
    final bd = ByteData(8);
    bd.setFloat64(0, v, Endian.big);
    writeAll(bd.buffer.asUint8List());
  }

  void _writeU16(int v) {
    writeByte((v >> 8) & 0xff);
    writeByte(v & 0xff);
  }

  void _writeU32(int v) {
    writeByte((v >> 24) & 0xff);
    writeByte((v >> 16) & 0xff);
    writeByte((v >> 8) & 0xff);
    writeByte(v & 0xff);
  }

  void _writeU64(int v) {
    // v must fit in 64-bit unsigned range; Dart int supports it.
    final hi = (v >> 32) & 0xffffffff;
    final lo = v & 0xffffffff;
    _writeU32(hi);
    _writeU32(lo);
  }
}

/// CBOR reader (definite-length only).
class _Reader {
  final Uint8List bytes;
  int pos = 0;

  _Reader(this.bytes);

  int? peekByte() => pos < bytes.length ? bytes[pos] : null;
  int readByte() {
    if (pos >= bytes.length) throw FormatException('Unexpected EOF');
    return bytes[pos++];
  }

  void ensureDone() {
    if (pos != bytes.length) {
      throw FormatException('Trailing bytes at position $pos');
    }
  }

  int _readUintN(int nbytes) {
    if (pos + nbytes > bytes.length) throw FormatException('Unexpected EOF');
    var v = 0;
    for (var i = 0; i < nbytes; i++) {
      v = (v << 8) | bytes[pos++];
    }
    return v;
  }

  int _readAiValue(int ai) {
    if (ai < 24) return ai;
    switch (ai) {
      case 24:
        return _readUintN(1);
      case 25:
        return _readUintN(2);
      case 26:
        return _readUintN(4);
      case 27:
        // 64-bit unsigned; may exceed 32-bit.
        final hi = _readUintN(4);
        final lo = _readUintN(4);
        return (hi << 32) | lo;
      default:
        throw FormatException(
          'Indefinite length or reserved AI not allowed: $ai',
        );
    }
  }

  int readLengthDefinite() {
    final ib = readByte();
    final ai = ib & 0x1f;
    if (ai == 31) throw FormatException('Indefinite length not allowed');
    return _readAiValue(ai);
  }

  int readUnsigned() {
    final ib = readByte();
    if ((ib >> 5) != 0) throw FormatException('Expected unsigned int');
    final ai = ib & 0x1f;
    if (ai == 31) throw FormatException('Indefinite not allowed');
    return _readAiValue(ai);
  }

  int readNegative() {
    final ib = readByte();
    if ((ib >> 5) != 1) throw FormatException('Expected negative int');
    final ai = ib & 0x1f;
    if (ai == 31) throw FormatException('Indefinite not allowed');
    final n = _readAiValue(ai);
    return -1 - n;
  }

  Uint8List readBytes() {
    final ib = readByte();
    if ((ib >> 5) != 2) throw FormatException('Expected byte string');
    final ai = ib & 0x1f;
    if (ai == 31) throw FormatException('Indefinite not allowed');
    final n = _readAiValue(ai);
    if (pos + n > bytes.length) throw FormatException('Unexpected EOF');
    final out = bytes.sublist(pos, pos + n);
    pos += n;
    return Uint8List.fromList(out);
  }

  String readText() {
    final ib = readByte();
    if ((ib >> 5) != 3) throw FormatException('Expected text string');
    final ai = ib & 0x1f;
    if (ai == 31) throw FormatException('Indefinite not allowed');
    final n = _readAiValue(ai);
    if (pos + n > bytes.length) throw FormatException('Unexpected EOF');
    final out = bytes.sublist(pos, pos + n);
    pos += n;
    return utf8.decode(out);
  }

  int readTagNumber() {
    final ib = readByte();
    if ((ib >> 5) != 6) throw FormatException('Expected tag');
    final ai = ib & 0x1f;
    if (ai == 31) throw FormatException('Indefinite not allowed');
    return _readAiValue(ai);
  }

  dynamic readSimple() {
    final ib = readByte();
    if ((ib >> 5) != 7) throw FormatException('Expected simple');
    final ai = ib & 0x1f;
    if (ai == 20) return false;
    if (ai == 21) return true;
    if (ai == 22) return null;
    throw FormatException('Unsupported simple value ai=$ai');
  }

  double readFloat64() {
    final ib = readByte();
    if ((ib >> 5) != 7 || (ib & 0x1f) != 27) {
      throw FormatException('Expected float64');
    }
    if (pos + 8 > bytes.length) throw FormatException('Unexpected EOF');
    final bd = ByteData.sublistView(bytes, pos, pos + 8);
    pos += 8;
    return bd.getFloat64(0, Endian.big);
  }

  dynamic readSimpleOrUnknown() {
    final ib = readByte();
    if ((ib >> 5) != 7) throw FormatException('Expected simple/float');
    final ai = ib & 0x1f;
    if (ai == 20) return false;
    if (ai == 21) return true;
    if (ai == 22) return null;
    if (ai == 23) return 'undefined'; // not DAG-CBOR, but some CBOR uses it
    if (ai == 24) {
      final v = _readUintN(1);
      return _Simple(v);
    }
    throw FormatException('Unsupported simple/float additional info: $ai');
  }
}

class _Simple {
  final int value;
  _Simple(this.value);
  @override
  String toString() => 'Simple($value)';
}
