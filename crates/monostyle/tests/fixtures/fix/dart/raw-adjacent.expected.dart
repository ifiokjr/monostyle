final pattern = r'\d+ {kept}';
final flags = r'multiline \n kept';
final both = '$pattern and $flags';


void main() {
  print(both);
}
