object Notes {
  val tricky: String =
    s"""value ${"brace"} with {braces} and "quotes"
second line done"""

  def show(): Unit = {
    println(tricky)
    println(s"inline ${tricky.length}")
  }
}
