// Clean idiomatic Kotlin with tricky raw JSON strings; nothing to fix.

fun payload(user: String): String {
    val json = """
{
  "user": "$user",
  "note": "keep {this} as-is",
  "amount": ${'$'}100
}
""".trimIndent()

    return json
}

fun main() {
    val p = payload("ada")

    println(p.length)
}
