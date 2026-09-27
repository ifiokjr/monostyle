// when expression over an Int with brace-bearing strings.

fun describe(x: Int): String {
    return when (x) {
        0 -> "zero {nothing}"
        1, 2 -> "small {value}"
        in 3..9 -> "medium $x"
        else -> "large {x}"
    }
}

fun main() {
    val d = describe(5)




    println(d)
}
