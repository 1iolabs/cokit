import '../../co_flutter.dart';

typedef Tag = (String, dynamic);

class Tags implements DagCborCodecProvider {
  final List<Tag> _tags;

  Tags(List<Tag> tags) : _tags = tags;

  dynamic first(String name) {
    return _tags
        .where((tag) => tag.$1 == name)
        .map((tag) => tag.$2)
        .firstOrNull;
  }

  String? firstString(String name) {
    return _tags
        .where((tag) => tag.$1 == name)
        .map((tag) => tag.$2)
        .whereType<String>()
        .firstOrNull;
  }

  static final codec = TagsDagCborCodec();
  @override
  DagCborCodec<dynamic> get dagCborCodec => codec;
}

class TagsDagCborCodec implements DagCborCodec<Tags> {
  @override
  fromDagCborValue(value) {
    if (value is List) {
      return Tags(value.map((tag) => (tag[0] as String, tag[1])).toList());
    }
    if (value == null) {
      return Tags([]);
    }
    throw Exception("TagsDagCborCodec.fromDagCborValue failed: ${value}");
  }

  @override
  toDagCborValue(value) {
    return value._tags.map((tag) => [tag.$1, tag.$2]).toList();
  }
}
