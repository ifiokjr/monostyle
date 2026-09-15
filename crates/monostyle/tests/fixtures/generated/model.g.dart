// GENERATED CODE - DO NOT MODIFY BY HAND
part of 'model.dart';
class ModelAdapter extends TypeAdapter<Model> {
  @override
  Model read(BinaryReader reader) {
    if (reader.readByte() == 0) { return Model(); }
    if (reader.readByte() == 1) { return Model(a: reader.readInt()); }
    if (reader.readByte() == 2) { return Model(a: reader.readInt(), b: reader.readInt()); }
    return Model();
  }
  @override
  void write(BinaryWriter writer, Model obj) {
    writer.writeByte(3);
    writer.writeInt(obj.a);
    writer.writeInt(obj.b);
  }
}
