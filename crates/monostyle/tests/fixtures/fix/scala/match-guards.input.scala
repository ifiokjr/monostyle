object Classifier {
  def name(value: Int): String = value match {
    case x if x < 0 => "negative {kept}"
    case 0 => "zero"
    case x if x % 2 == 0 => "even"
    case _ => "odd"
  }
}
