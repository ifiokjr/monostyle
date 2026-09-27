fun label(code: Int): String = when (code) {
    0 -> "zero"
    1 -> "one"
    else -> "many"
}

fun guarded(code: Int): String = when {
    code < 0 -> "negative"
    code < 10 -> "small"
    else -> "large"
}
