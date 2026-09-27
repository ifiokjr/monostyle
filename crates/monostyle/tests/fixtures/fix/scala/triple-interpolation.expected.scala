object Demo {
  def render(name: String): String = {
    val text = s"""multi-line with ${name}
and {braces} inside
second line ${name} done"""
    println(text)

    text
  }

  def main(args: Array[String]): Unit = {
    println(render("world"))
  }
}
