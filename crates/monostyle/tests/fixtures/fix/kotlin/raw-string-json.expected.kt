// Raw JSON string with dollar templates; blank-line defect lives in main.

fun buildQuery(name: String): String {
    val query = """
{
  "user": "$name",
  "roles": ["admin"],
  "note": "use ${'$'}name carefully {braces}"
}
""".trimIndent()

    return query
}

fun main() {
    val out = buildQuery("ada")

    println(out)
}
