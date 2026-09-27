object Picker {
  def pick(key: String): String = {
    val label = key match {
      case "a" => "alpha {braced}"
      case "b" => "beta {body}"
      case other => s"other ${other}"
    }
    println(label)

    label
  }

  def main(args: Array[String]): Unit = {
    println(pick("a"))
  }
}
